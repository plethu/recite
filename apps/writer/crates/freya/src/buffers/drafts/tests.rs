use super::*;

#[test]
fn unchanged_editor_notifications_do_no_document_comparisons_or_captures() {
    for rows in [10, 1_000, 100_000] {
        let original = "# Source 🦊\r\n".repeat(rows);
        let mut rope = Rope::from_str(&original);
        let original_draft = DraftSnapshot::from(original.clone());
        let mut capture = SourceDraft::default();
        let comparisons = std::cell::Cell::new(0);
        let current = |text| {
            comparisons.set(comparisons.get() + 1);
            text
        };
        assert!(
            capture
                .capture(&rope, || current(&original_draft))
                .is_none()
        );
        comparisons.set(0);
        for _ in 0..100 {
            assert!(
                capture
                    .capture(&rope.clone(), || current(&original_draft))
                    .is_none()
            );
        }
        assert_eq!(comparisons.get(), 0);
        rope.insert(rope.len_chars() / 2, "é💬");
        let updated = capture.capture(&rope, || current(&original_draft)).unwrap();
        assert!(updated.matches_rope(&rope));
        assert_eq!(comparisons.get(), 1);
        for _ in 0..100 {
            assert!(capture.capture(&rope, || current(&updated)).is_none());
        }
        assert_eq!(comparisons.get(), 1);
        let identical = Rope::from_str(&String::from(&rope));
        assert!(capture.capture(&identical, || current(&updated)).is_none());
        assert_eq!(comparisons.get(), 2);
        capture.previous = None;
        assert!(capture.capture(&rope, || current(&updated)).is_none());
        assert_eq!(comparisons.get(), 3);
    }
}

#[test]
fn source_harvest_and_view_changes_keep_only_the_active_widget_text() {
    let text = format!(
        "{}{}",
        recite_writer_model::FIXTURE,
        "# large source padding\r\n".repeat(10_000)
    );
    let mut model = Workbench::new(&text).unwrap();
    let passage = model.document().passage_snapshot().unwrap()[0].id.clone();
    let prose = model.draft().to_owned();
    assert_eq!(super::super::prose_draft(&model), prose);
    model.select(View::Source).unwrap();
    let mut source = Rope::from_str(&text);
    assert_eq!(super::super::prose_draft(&model).to_owned().capacity(), 0);
    harvest_draft(&mut model, &source, "unused prose");
    assert!(!model.has_draft());
    let captured = model.recovery();
    source.insert(source.len_chars(), "# unapplied\n");
    harvest_draft(&mut model, &source, "unused prose");
    assert!(model.has_draft());
    assert!(model.draft_snapshot().matches_rope(&source));
    assert_eq!(super::super::prose_draft(&model), "");
    assert_eq!(captured.draft_snapshot().as_str(), text);
    model.discard();
    model.select(View::Passage(passage)).unwrap();
    assert_eq!(super::super::prose_draft(&model), prose);
    harvest_draft(&mut model, &source, "Edited prose");
    assert_eq!(model.draft(), "Edited prose");
    assert!(model.has_draft());
    model.discard();
    model.select(View::Source).unwrap();
    assert_eq!(model.draft(), text);
    assert_eq!(super::super::prose_draft(&model), "");
}
