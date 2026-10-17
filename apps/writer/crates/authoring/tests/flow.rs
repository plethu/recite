use recite_writer_model::{View, WRITER_EXAMPLES, scene_links};

#[test]
fn inspection_restores_context_without_running_the_story() -> Result<(), Box<dyn std::error::Error>>
{
    let mut session = WRITER_EXAMPLES[0].open()?;
    let source = session.document().source().to_owned();
    session.inspect_block("accept_search")?;
    session.select(View::Source)?;
    session.show_script()?;
    assert_eq!(session.selected_block()?.as_deref(), Some("accept_search"));
    session.inspect_block("relay_desk")?;
    assert_eq!(session.view(), &View::Block("relay_desk".into()));
    assert_eq!(session.document().source(), source);
    assert!(session.preview_page().is_none());
    assert!(session.inspect_block("missing").is_err());
    assert_eq!(session.selected_block()?.as_deref(), Some("relay_desk"));
    Ok(())
}

#[test]
fn map_preserves_choices_and_shared_destinations() -> Result<(), Box<dyn std::error::Error>> {
    let session = WRITER_EXAMPLES[1].open()?;
    let blocks = session.document().script()?;
    let links = scene_links(&blocks);
    assert_eq!(
        links.iter().filter(|link| link.origin == "alarm").count(),
        2
    );
    let origins: Vec<_> = links
        .iter()
        .filter(|link| link.destination == "gatehouse")
        .map(|link| link.origin.as_str())
        .collect();
    assert_eq!(origins, ["rooftops", "market"]);
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block.id == "gatehouse")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn return_edges_follow_topology_and_forward_edges_keep_their_meaning()
-> Result<(), Box<dyn std::error::Error>> {
    let hub = WRITER_EXAMPLES[0].open()?.document().script()?;
    let links = scene_links(&hub);
    assert!(
        links
            .iter()
            .filter(|l| l.destination == "relay_desk")
            .all(|l| l.returning)
    );
    assert!(
        links
            .iter()
            .filter(|l| l.origin == "relay_desk")
            .all(|l| !l.returning)
    );
    let waterfall = WRITER_EXAMPLES[1].open()?.document().script()?;
    assert!(scene_links(&waterfall).iter().all(|l| !l.returning));
    Ok(())
}

#[test]
fn return_classification_starts_at_the_declared_entry_even_if_it_is_last()
-> Result<(), Box<dyn std::error::Error>> {
    let document =
        recite_writer_model::Document::new(":: topic\n-> hub\n:: hub default\n-> topic\n")?;
    let links = scene_links(&document.script()?);
    assert!(
        links
            .iter()
            .find(|l| l.origin == "topic")
            .ok_or("topic")?
            .returning
    );
    assert!(
        !links
            .iter()
            .find(|l| l.origin == "hub")
            .ok_or("hub")?
            .returning
    );
    Ok(())
}

#[test]
fn return_classification_preserves_duplicate_origins_and_unresolved_slots() {
    use recite_writer_model::{ScriptBlock, ScriptEntry};
    let block = |id: &str, is_default, targets: &[&str]| ScriptBlock {
        id: id.into(),
        is_default,
        entries: targets
            .iter()
            .map(|target| ScriptEntry::Jump((*target).into()))
            .collect(),
    };
    let blocks = [
        block("topic", false, &["hub", "hub"]),
        block("hub", true, &["topic", "hub", "missing", "END"]),
        block("topic", false, &["hub", "other"]),
        block("other", false, &["topic"]),
    ];
    let links = scene_links(&blocks);
    assert_eq!(
        links.iter().map(|link| link.returning).collect::<Vec<_>>(),
        [true, true, false, true, false, false, true, false, true]
    );
}

#[test]
fn return_classification_keeps_large_linear_traversal_off_the_call_stack() {
    use recite_writer_model::{ScriptBlock, ScriptEntry};
    let blocks: Vec<_> = (0..20_000)
        .map(|i| ScriptBlock {
            id: format!("beat_{i}"),
            is_default: i == 0,
            entries: vec![ScriptEntry::Jump(if i == 19_999 {
                "beat_0".into()
            } else {
                format!("beat_{}", i + 1)
            })],
        })
        .collect();
    let links = scene_links(&blocks);
    assert_eq!(links.len(), blocks.len());
    assert!(links[..19_999].iter().all(|link| !link.returning));
    assert!(links[19_999].returning);
}

#[test]
fn ending_badges_match_flow_for_nested_choices_jumps_and_duplicate_names()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_writer_model::{Document, scene_endings};
    use std::collections::BTreeSet;
    for source in [
        ":: start default\n-> END\n",
        ":: start\n:if ready()\n  -> END\n",
        ":: start\n? finish@11111111111111111111\n  Finish.\n  -> END\n? stay@22222222222222222222\n  Stay.\n  -> start\n",
        ":: start\n-> other\n:: start\n-> END\n:: other\n-> missing\n",
        ":: first\n> line@11111111111111111111\n  A line mentioning END.\n-> missing\n",
    ] {
        let blocks = Document::new(source)?.script()?;
        let links = scene_links(&blocks);
        let expected: BTreeSet<_> = links
            .iter()
            .filter(|l| l.destination == "END")
            .map(|l| l.origin.as_str())
            .collect();
        assert_eq!(scene_endings(&blocks), expected, "{source}");
    }
    Ok(())
}
