use super::*;
use crate::attention::EditingTarget;
use crate::tests::support;
use std::{
    fs,
    sync::{atomic, mpsc},
};
use tokio::sync::mpsc as async_mpsc;
#[path = "failure/tests.rs"]
mod failure;

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n? reply@22222222222222222222\n  Continue.\n  -> END\n";
const PO: &str = "#. file: scene.recite\n#. block: start\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";
const SCREENS: &[Screen] = &[
    Screen::Write,
    Screen::Preview,
    Screen::Rules,
    Screen::Declarations,
    Screen::Build,
    Screen::Rename,
    Screen::Disk,
    Screen::Localise,
    Screen::Translations,
    Screen::Updates,
    Screen::Compare,
    Screen::Entry,
];

fn fixture() -> (tempfile::TempDir, Writer) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\nschema='schema.json'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("schema.json"),
        r#"{"schema_version":1}"#,
    )
    .unwrap();
    fs::write(directory.path().join("fr.po"), PO).unwrap();
    let writer = support::open(&directory).unwrap();
    (directory, writer)
}
fn location(screen: Screen) -> Location {
    Location {
        screen,
        document: "scene.recite".into(),
        catalogue: Some("fr.po".into()),
        beat: Some("start".into()),
        passage: Some(
            if screen == Screen::Rules {
                "22222222222222222222"
            } else {
                "11111111111111111111"
            }
            .into(),
        ),
        entry: (screen == Screen::Entry).then(|| "11111111111111111111".into()),
        ..Location::default()
    }
}
fn connect(
    writer: &mut Writer,
) -> (
    tokio::runtime::Runtime,
    async_mpsc::UnboundedReceiver<worker::Job>,
) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    (runtime, jobs)
}
fn activate(writer: &mut Writer, location: Location) -> mpsc::Receiver<Result<(), String>> {
    let (reply, waiting) = mpsc::channel();
    let incoming = recite_writer_session::activation::IncomingRoute {
        project: None,
        route: Some(format!("recite://writer{location}")),
        document: None,
        editor: None,
        cancelled: Arc::new(atomic::AtomicBool::new(false)),
        reply,
    };
    writer.navigate_activation(location, incoming).unwrap();
    waiting
}
fn publish(writer: &mut Writer, job: worker::Job) {
    let completed = job.execute();
    let request = completed.request.clone();
    if let Some(retired) = writer.accept_navigation(completed) {
        retired.retire().unwrap();
    }
    writer.navigation_retired(&request);
}
#[test]
fn every_linked_screen_has_the_same_destination_on_cold_and_existing_window_navigation() {
    for &screen in SCREENS {
        for cold in [true, false] {
            let (_directory, mut writer) = fixture();
            let location = location(screen);
            if cold {
                writer.follow_location(&location).unwrap();
            } else {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                let (sender, mut jobs) = async_mpsc::unbounded_channel();
                writer.navigation.sender = Some(sender);
                writer.navigation.runtime = Some(runtime.handle().clone());
                let (reply, waiting) = mpsc::channel();
                let incoming = recite_writer_session::activation::IncomingRoute {
                    project: None,
                    route: Some(format!("recite://writer{location}")),
                    document: None,
                    editor: None,
                    cancelled: Arc::new(atomic::AtomicBool::new(false)),
                    reply,
                };
                writer
                    .navigate_activation(location.clone(), incoming)
                    .unwrap();
                let first = jobs.try_recv().unwrap().execute();
                if let Some(retired) = writer.accept_navigation(first) {
                    retired.retire().unwrap();
                }
                assert!(
                    waiting.try_recv().is_err(),
                    "{screen:?} must not finish while destination preparation is queued"
                );
                assert!(writer.navigation.busy());
                support::finish_navigation(&mut writer, &mut jobs).unwrap();
                assert!(
                    waiting.try_recv().unwrap().is_ok(),
                    "{screen:?}: {}",
                    writer.status
                );
            }
            let actual = writer.current_location();
            assert_eq!(
                writer
                    .attention
                    .edit_request()
                    .map(|request| request.target()),
                (screen == Screen::Write)
                    .then(|| EditingTarget::capture(&writer))
                    .flatten()
                    .as_ref(),
                "only explicit writing passages request focus, cold={cold}, screen={screen:?}"
            );
            assert_eq!(actual.screen, screen, "cold={cold}: {}", writer.status);
            assert_eq!(actual.catalogue.as_deref(), Some("fr.po"));
            if !cold
                && matches!(
                    screen,
                    Screen::Declarations | Screen::Build | Screen::Rename
                )
            {
                assert_eq!(
                    writer
                        .navigation
                        .history
                        .step(false)
                        .unwrap()
                        .location()
                        .screen,
                    Screen::Write,
                    "a composed destination must record after its catalogue is installed"
                );
            }
            if screen == Screen::Entry {
                assert_eq!(actual.entry, location.entry);
            }
            assert!(!writer.session.read_only());
            assert_eq!(writer.session.model().document().source(), SOURCE);
        }
    }
}

#[test]
fn source_update_links_use_logical_change_ordinals_even_when_the_first_change_needs_many_pages() {
    let (_directory, mut writer) = fixture();
    writer
        .session
        .edit(|model| {
            model.set_draft("é".repeat(12_000));
            model.apply()
        })
        .unwrap();
    let mut target = location(Screen::Updates);
    target.page = 1;
    writer.follow_location(&target).unwrap();
    assert_eq!(writer.current_location().screen, Screen::Updates);
    assert_eq!(writer.current_location().page, 1);
}
