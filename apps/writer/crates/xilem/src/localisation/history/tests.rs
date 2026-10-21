use super::*;
use crate::localisation::tests::SOURCE;
use crate::{tests::support, workspaces::tests::ToolDriver};
use recite_writer_session::location::Screen as LinkedScreen;
use std::fs;
use tokio::sync::mpsc;

#[test]
fn ordinary_catalogue_history_restores_query_filter_page_and_entry_after_checked_publication() {
    let directory = support::project(SOURCE).unwrap();
    let po = (1..=18)
        .map(|index| format!("msgctxt \"{index:020x}\"\nmsgid \"Hello {index}\"\nmsgstr \"\"\n\n"))
        .collect::<String>();
    fs::write(directory.path().join("fr.po"), po).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut tools = ToolDriver::new(&mut writer);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.open_localisation();
    writer.localisation.path = directory.path().join("fr.po").display().to_string();
    writer.open_catalogue_path();
    tools.finish(&mut writer);
    for query in ["H", "Hello"] {
        writer.set_catalogue_query(query.into());
    }
    writer.localisation.query.filter = Filter::Attention;
    writer.search_catalogue();
    tools.finish(&mut writer);
    writer.localisation.query.page = 1;
    writer.catalogue_action(work::Action::Query);
    tools.finish(&mut writer);
    let queue = writer.current_location();
    assert_eq!(
        (
            queue.screen,
            queue.query.as_str(),
            queue.attention,
            queue.page
        ),
        (LinkedScreen::Translations, "Hello", true, 1)
    );
    let key = EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id: writer.localisation.queue.rows[0].id,
    };
    writer.inspect_translation(&key);
    tools.finish(&mut writer);
    let entry = writer.current_location();
    assert_eq!(entry.screen, LinkedScreen::Entry);
    assert_eq!(
        writer.navigation.history.step(false).unwrap().location(),
        &queue
    );
    writer.history_step(false);
    let completed = jobs.try_recv().unwrap().execute();
    writer.cancel_navigation();
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.current_location(), entry);
    assert_eq!(
        writer.navigation.history.step(false).unwrap().location(),
        &queue
    );
    for screen in [
        LinkedScreen::Translations,
        LinkedScreen::Translations,
        LinkedScreen::Write,
    ] {
        writer.history_step(false);
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
        assert_eq!(
            writer.current_location().screen,
            screen,
            "{}",
            writer.status
        );
        if writer.current_location().catalogue.is_some() {
            assert_eq!(writer.current_location(), queue);
        }
    }
    assert!(writer.navigation.history.step(false).is_none());
    for _ in 0..3 {
        writer.history_step(true);
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
    }
    assert_eq!(writer.current_location(), entry);
    assert!(writer.navigation.history.step(true).is_none());
    writer.history_step(false);
    let completed = jobs.try_recv().unwrap().execute();
    writer.return_to_writing();
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.current_location().screen, LinkedScreen::Write);
    assert_eq!(
        writer.navigation.history.step(false).unwrap().location(),
        &entry
    );
}
