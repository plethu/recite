use super::*;

#[test]
fn history_moves_only_after_accepted_navigation_and_stale_typing_keeps_the_previous_cursor() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.inspect("second".into());
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.current_location().beat.as_deref(), Some("second"));
    writer.history_step(false);
    assert_eq!(writer.current_location().beat.as_deref(), Some("second"));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.current_location().beat.as_deref(), Some("first"));
    assert!(writer.navigation.history.step(false).is_none());
    let draft = writer.session.model().draft().to_owned();
    writer.history_step(true);
    let completed = fixture.jobs.try_recv().unwrap().execute();
    writer.change(|model| {
        model.set_draft("Typed while moving forward.".into());
        Ok(())
    });
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.current_location().beat.as_deref(), Some("first"));
    assert!(writer.navigation.history.step(true).is_some());
    assert_eq!(
        writer.session.model().draft(),
        "Typed while moving forward."
    );
    writer.change(|model| {
        model.set_draft(draft);
        Ok(())
    });
    writer.history_step(true);
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.current_location().beat.as_deref(), Some("second"));
    assert!(writer.navigation.history.step(true).is_none());
}
