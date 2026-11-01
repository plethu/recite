use super::*;
use crate::localisation::{Destination, tests::form};
use recite_writer_model::View;

#[test]
fn ordinary_bilingual_entry_preserves_unapplied_prose_and_source_recovery_without_editor_launch() {
    for source_view in [false, true] {
        let (_directory, mut writer) = fixture();
        let key = row_key(&writer, 0);
        writer.return_to_writing();
        writer.change(|model| {
            if source_view {
                model.select(View::Source)?;
            }
            model.set_draft("Unapplied private manuscript.".into());
            Ok(())
        });
        let before = writer.session.model().recovery();
        let source = writer.session.model().document().source_snapshot();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        writer.open_localisation();
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
        assert!(
            writer.workspaces.screen == crate::workspaces::Screen::Localisation,
            "{}",
            writer.status
        );
        assert_eq!(writer.session.model().recovery(), before);
        assert_eq!(writer.session.model().document().source_snapshot(), source);
        assert!(matches!(
            writer.localisation.destination,
            Destination::Passage
        ));
        assert_eq!(
            writer.localisation.passage.as_ref().unwrap().rows[0]
                .passage()
                .unwrap()
                .source
                .text,
            "Hello."
        );
        assert!(!writer.current_location().source);
        assert_eq!(
            writer.current_location().view,
            Some(recite_config::WriterView::Script)
        );
        assert!(!writer.external.busy());
        writer.update_translation(&key, 0, "Independent translation.".into());
        assert_eq!(form(&writer, &key), "Independent translation.");
        writer.return_to_writing();
        assert_eq!(writer.session.model().recovery(), before);
    }
}
#[test]
fn cancelled_bilingual_entry_returns_both_owners_and_keeps_the_previous_surface() {
    let (_directory, mut writer) = fixture();
    writer.return_to_writing();
    let before = writer.session.model().recovery();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.open_localisation();
    let completed = jobs.try_recv().unwrap().execute();
    writer.cancel_navigation();
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.session.model().recovery(), before);
    assert!(!writer.session.checked_out() && !writer.localisation.working());
    assert!(writer.workspaces.screen == crate::workspaces::Screen::Write);
}

#[test]
fn ambiguous_accepted_source_refuses_bilingual_entry_without_displacing_source_recovery() {
    let (_directory, mut writer) = fixture();
    writer.return_to_writing();
    writer.change(|model| {
        model.select(View::Source)?;
        model.set_draft(format!("{SOURCE}\n:: start\n-> END\n"));
        model.apply()?;
        model.set_draft("Still-unapplied source recovery.".into());
        Ok(())
    });
    let before = writer.session.model().recovery();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.open_localisation();
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert_eq!(writer.session.model().recovery(), before);
    assert!(!writer.session.checked_out() && !writer.localisation.working());
    assert!(writer.workspaces.screen == crate::workspaces::Screen::Write);
    assert_ne!(writer.status, "Ready");
}
