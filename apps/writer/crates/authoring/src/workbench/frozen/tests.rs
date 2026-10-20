use super::*;
use std::sync::Arc;

#[test]
fn frozen_presentation_shares_accepted_analysis_and_field_state_without_edit_history() {
    let mut model = crate::WRITER_EXAMPLES[0].open().unwrap();
    model.select(View::Source).unwrap();
    model.set_draft(format!("{}\n# accepted edit\n", model.document().source()));
    model.apply().unwrap();
    assert!(model.document().can_undo());
    model.set_draft(format!("{}# live field\n", model.document().source()));
    let frozen = model.freeze();
    assert!(model.navigation_stamp().is_current(frozen.model()));
    assert!(Arc::ptr_eq(
        &model.document().source_snapshot(),
        &frozen.model().document().source_snapshot()
    ));
    assert!(!frozen.model().document().can_undo());
    assert!(model.document().can_undo());
    assert_eq!(frozen.model().draft(), model.draft());
}
