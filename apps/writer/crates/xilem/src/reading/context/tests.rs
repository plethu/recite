use crate::{
    Writer,
    tests::support::{click, finish_navigation, labels, open, project},
};
use recite_writer_model::View;
use tokio::sync::mpsc;

const SOURCE: &str = ":: first default\n> greeting@11111111111111111111\n  First.\n? reply@22222222222222222222\n  Where?\n  -> second\n:: second\n> answer@33333333333333333333\n  Second.\n-> END\n";

fn pinned_text(writer: &Writer) -> String {
    format!(
        "{:?}",
        writer
            .reading_context
            .pinned
            .as_ref()
            .unwrap()
            .snapshot
            .block()
    )
}

#[test]
fn pin_applies_outgoing_text_and_remains_a_snapshot_through_navigation() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Accepted greeting.".into());
        Ok(())
    });
    click(&mut writer, &["Pin reference"]).unwrap();
    assert!(!writer.session.model().has_draft());
    let pinned = pinned_text(&writer);
    assert!(pinned.contains("Accepted greeting."));
    writer.trial.visible = true;
    click(&mut writer, &["Second · end"]).unwrap();
    assert!(
        !writer.trial.visible,
        "choosing a beat must reveal its writing"
    );
    assert_eq!(writer.session.model().view(), &View::Block("second".into()));
    assert_eq!(pinned_text(&writer), pinned);
    click(&mut writer, &["Hide reference"]).unwrap();
    assert!(writer.reading_context.pinned.as_ref().unwrap().collapsed);
    click(&mut writer, &["Unpin reference"]).unwrap();
    assert!(writer.reading_context.pinned.is_none());
}

#[test]
fn invalid_and_superseded_pin_requests_preserve_the_previous_reference() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    click(&mut writer, &["Pin reference"]).unwrap();
    let original = pinned_text(&writer);
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    click(&mut writer, &["Pin reference"]).unwrap();
    assert_eq!(writer.session.model().draft(), "-> syntax");
    assert_eq!(pinned_text(&writer), original);

    writer.change(|model| {
        model.set_draft("Captured greeting.".into());
        Ok(())
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.pin_reference();
    let completed = jobs.try_recv().unwrap().execute();
    writer.change(|model| {
        model.set_draft("Newer greeting.".into());
        Ok(())
    });
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.session.model().draft(), "Newer greeting.");
    assert_eq!(pinned_text(&writer), original);
    assert!(!writer.navigation.busy());
    assert!(jobs.try_recv().is_err());
    writer.pin_reference();
    finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(pinned_text(&writer).contains("Newer greeting."));
}

#[test]
fn destination_preview_leaves_an_invalid_draft_open_and_edit_uses_normal_admission() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    let view = writer.session.model().view().clone();
    let rows = writer.reading.rows.clone();
    click(&mut writer, &["→ Second"]).unwrap();
    assert!(writer.destination_expanded("second"));
    assert_eq!(writer.session.model().view(), &view);
    assert_eq!(writer.session.model().draft(), "-> syntax");
    assert!(std::sync::Arc::ptr_eq(&rows, &writer.reading.rows));
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|text| text == "Second.")
    );
    click(&mut writer, &["Edit Second"]).unwrap();
    assert_eq!(writer.session.model().view(), &view);
    assert_eq!(writer.session.model().draft(), "-> syntax");
    click(&mut writer, &["▾ Second"]).unwrap();
    assert!(!writer.destination_expanded("second"));
}
