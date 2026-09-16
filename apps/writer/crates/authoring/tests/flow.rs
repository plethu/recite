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
