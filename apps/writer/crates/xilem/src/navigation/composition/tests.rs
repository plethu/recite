use super::*;
use crate::attention::EditingTarget;
use crate::map::Workspace;

#[test]
fn queued_native_focus_and_close_cannot_retire_the_composing_passage() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let selected = writer.session.model().view().clone();
    let target = EditingTarget::passage(writer, "22222222222222222222".into());
    let preedit = writer.attention.begin_preedit();
    writer.native_field_focus(Some(target.clone()));
    assert_eq!(writer.session.model().view(), &selected);
    writer.request_close();
    assert!(writer.phase == crate::Phase::Writing);
    drop(preedit);
    writer.native_field_focus(Some(target.clone()));
    assert_eq!(writer.session.model().view(), &target.view());
}

#[test]
fn preedit_started_during_reload_or_map_preparation_keeps_the_original_passage_open() {
    for target in [Target::Reload, Target::Workspace(Workspace::Map)] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let reloading = matches!(target, Target::Reload);
        let original = writer.session.model().document().source_snapshot();
        let selected = writer.session.model().view().clone();
        writer.navigate(target);
        let completed = fixture.jobs.try_recv().unwrap().execute();
        let preedit = writer.attention.begin_preedit();
        let request = completed.request.clone();
        if let Some(retired) = writer.accept_navigation(completed) {
            retired.retire().unwrap();
        }
        writer.navigation_retired(&request);
        assert_eq!(writer.session.model().view(), &selected);
        assert!(Arc::ptr_eq(
            &original,
            &writer.session.model().document().source_snapshot()
        ));
        assert_eq!(writer.visible_workspace(), Workspace::Script);
        assert!(writer.status.contains("composition"), "{}", writer.status);
        assert!(!writer.navigation.busy());
        if reloading {
            assert!(writer.recovery_copy.as_ref().unwrap().is_file());
        }
        drop(preedit);
        writer.request_close();
        assert!(writer.phase != crate::Phase::Writing);
    }
}
