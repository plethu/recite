use recite_writer_model::{ScriptEntry, View, WRITER_EXAMPLES, WorkbenchError};
use recite_writer_session::reading::{Excerpt, ReferenceSnapshot};

#[test]
fn pinned_reference_is_an_accepted_snapshot_independent_of_edits_and_navigation() {
    let mut model = WRITER_EXAMPLES[0].open().unwrap();
    let pinned = ReferenceSnapshot::capture(&model).unwrap().unwrap();
    let original = pinned.block().clone();
    assert_eq!(original.id, "relay_desk");
    assert_eq!(pinned.document(), model.document().key());
    model.set_draft("New greeting.".into());
    assert!(matches!(
        ReferenceSnapshot::capture(&model),
        Err(WorkbenchError::DraftPending)
    ));
    assert_eq!(pinned.block(), &original);
    model.apply().unwrap();
    let updated = ReferenceSnapshot::capture(&model).unwrap().unwrap();
    assert_ne!(updated.block(), &original);
    model.select(View::Block("missing_courier".into())).unwrap();
    assert_eq!(pinned.block(), &original);
    assert_eq!(updated.block().id, "relay_desk");
}

#[test]
fn reference_without_a_selected_beat_is_empty() {
    let mut model = WRITER_EXAMPLES[0].open().unwrap();
    model.select(View::Source).unwrap();
    assert!(ReferenceSnapshot::capture(&model).unwrap().is_none());
}

#[test]
fn excerpts_follow_source_order_through_groups_and_have_one_shared_work_budget() {
    let items = vec![
        ScriptEntry::Source("first".into()),
        ScriptEntry::Group {
            heading: "condition".into(),
            entries: vec![
                ScriptEntry::Effect("effect".into()),
                ScriptEntry::Group {
                    heading: "nested".into(),
                    entries: vec![ScriptEntry::Jump("inside".into())],
                },
            ],
        },
        ScriptEntry::Jump("last".into()),
    ];
    let mut excerpt = Excerpt::new(&items);
    let rows = excerpt
        .by_ref()
        .map(|row| (caption(row.entry), row.depth))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            ("first", 0),
            ("condition", 0),
            ("effect", 1),
            ("nested", 1),
            ("inside", 2),
            ("last", 0)
        ]
    );
    assert!(!excerpt.limited());

    // A deep group consumes the same budget as a passage: neither depth nor a
    // large following sibling list can cause an unbounded preview traversal.
    let mut deep = ScriptEntry::Jump("hidden".into());
    for _ in 0..128 {
        deep = ScriptEntry::Group {
            heading: "deep".into(),
            entries: vec![deep],
        };
    }
    let mut wide = vec![deep];
    wide.extend((0..10_000).map(|i| ScriptEntry::Source(i.to_string())));
    let mut excerpt = Excerpt::new(&wide);
    let visible = excerpt
        .by_ref()
        .map(|row| caption(row.entry))
        .collect::<Vec<_>>();
    assert_eq!(visible.len(), 32);
    assert!(visible.iter().all(|text| *text == "deep"));
    assert!(excerpt.limited());
    assert!(excerpt.next().is_none());
    let exact = (0..32)
        .map(|i| ScriptEntry::Source(i.to_string()))
        .collect::<Vec<_>>();
    let mut excerpt = Excerpt::new(&exact);
    assert_eq!(excerpt.by_ref().count(), 32);
    assert!(
        !excerpt.limited(),
        "an exactly full preview does not omit content"
    );
}

fn caption(entry: &ScriptEntry) -> &str {
    match entry {
        ScriptEntry::Group { heading, .. } => heading,
        ScriptEntry::Jump(text) | ScriptEntry::Effect(text) | ScriptEntry::Source(text) => text,
        ScriptEntry::Passage(passage) => &passage.text,
    }
}
