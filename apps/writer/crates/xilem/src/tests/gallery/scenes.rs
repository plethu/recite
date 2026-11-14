//! The catalogue: empty and representative states of every surface, and the transitions between them.
//!
//! Scene ids are stable: comparisons match baseline frames by id, so rename one only together with
//! a new baseline. Add a scene when a surface or state ships; reviewers cannot judge what is absent.
mod stills;
mod transitions;
pub(super) use stills::stills;
pub(super) use transitions::transitions;

use super::record::{Meta, Recorder};
use super::stage::{Frame, Stage};
use super::{Settings, Variant};
use crate::Writer;
use crate::app::{App, tests::gallery_page};
use crate::session::Session;
use crate::specimen::tests::gallery as specimen_gallery;
use recite_config::WriterCommand;
use recite_writer_app::shell::Search;
use recite_writer_session::ProjectSession;
use recite_writer_session::location::{Location, Screen};
use std::fs;
use std::path::Path;
use xilem::WidgetView;
use xilem::masonry::widgets::Passthrough;

const SCHEMA: &str =
    include_str!("../../../../../../../fixtures/schema/valid/writer_examples.json");
const RELAY: &str =
    include_str!("../../../../../../../fixtures/recite/valid/writer_examples/relay_hub.recite");
const FLOODGATE: &str = include_str!(
    "../../../../../../../fixtures/recite/valid/writer_examples/floodgate_waterfall.recite"
);
const TRAM: &str = include_str!(
    "../../../../../../../fixtures/recite/valid/writer_examples/last_tram_linear.recite"
);
// A short scene whose catalogue covers translated, context-variant and missing entries.
const HARBOUR: &str = ":: harbour\n> @ffee6p8\n  The last ferry is already boarding.\n> @rthh7ab\n  Bring the lamp; the pier lights are out again.\n> @bqyvpqq\n  Nobody has written this line in French yet.\n-> END\n";
const FRENCH: &str = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"ffee6p8&weary\"\nmsgid \"The ferry is boarding.\"\nmsgstr \"Le ferry embarque.\"\n\nmsgctxt \"ffee6p8\"\nmsgid \"The last ferry is already boarding.\"\nmsgstr \"Le dernier ferry embarque déjà.\"\n\nmsgctxt \"rthh7ab\"\nmsgid \"Bring the lamp; the pier lights are out again.\"\nmsgstr \"Prends la lampe ; les feux du quai sont encore éteints.\"\n";

/// A project has one default beat; the other scenes keep theirs as ordinary beats.
fn secondary(source: &str) -> String {
    source.replacen(" default\n", "\n", 1)
}

/// A project folder with several scenes, a schema and a French catalogue.
fn sample() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("sample project folder");
    let path = directory.path();
    let files = [
        (
            "recite.project.toml",
            "format_version = 1\n[project]\ncontent_set = \"harbour\"\nversion = \"1\"\nschema = \"schema.json\"\n",
        ),
        ("schema.json", SCHEMA),
        ("relay_hub.recite", RELAY),
        ("floodgate_waterfall.recite", &secondary(FLOODGATE)),
        ("last_tram_linear.recite", &secondary(TRAM)),
        ("harbour.recite", HARBOUR),
        ("fr.po", FRENCH),
    ];
    for (name, contents) in files {
        fs::write(path.join(name), contents).expect("sample project file");
    }
    directory
}

/// The project a new writer gets from the welcome screen's create action.
fn starter() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("starter project folder");
    recite_config::create_starter_project(&directory.path().join("harbour"), "Harbour")
        .expect("starter project");
    directory
}

fn open(path: &Path, document: &str) -> Writer {
    let requested = (!document.is_empty()).then(|| Path::new(document));
    let session = ProjectSession::open(path, requested).expect("open gallery project");
    Writer::from_session(Session::Project(Box::new(session)), None)
}

fn examples(document: &str) -> Writer {
    let mut writer = Writer::from_session(Session::examples().expect("examples"), None);
    writer
        .follow_location(&Location {
            document: document.into(),
            ..Location::default()
        })
        .expect("open example");
    writer
}

fn stage(
    mut app: App,
    variant: Variant,
) -> Stage<App, impl WidgetView<App, Widget = Passthrough> + use<>> {
    variant.configure(&mut app.preferences.config.writer);
    Stage::new(app, gallery_page, App::gallery_look, variant.frame)
}

fn visit<V: WidgetView<App, Widget = Passthrough>>(stage: &mut Stage<App, V>, location: Location) {
    let result = stage
        .state
        .writer
        .as_mut()
        .map(|writer| writer.follow_location(&location));
    if let Some(Err(error)) = result {
        stage.warnings.push(format!("location {location}: {error}"));
    }
    stage.settle();
}

fn command<V: WidgetView<App, Widget = Passthrough>>(
    stage: &mut Stage<App, V>,
    command: WriterCommand,
) {
    stage.act(|app| app.run_command(command));
}

fn draft<V: WidgetView<App, Widget = Passthrough>>(stage: &mut Stage<App, V>, text: &str) {
    stage.act(|app| {
        if let Some(writer) = app.writer.as_mut() {
            writer.change(|model| {
                model.set_draft(text.into());
                Ok(())
            });
        }
    });
}

/// One transition step: the settled frame, then any requested motion frames after it.
fn step<V: WidgetView<App, Widget = Passthrough>>(
    stage: &mut Stage<App, V>,
    record: &mut Recorder,
    settings: &Settings,
    caption: &str,
) {
    stage.shot(record, caption);
    let mut elapsed = 0;
    for &ms in &settings.motion {
        stage.animate(ms.saturating_sub(elapsed));
        elapsed = ms;
        stage.shot(record, &format!("{caption} (+{ms} ms)"));
    }
}

fn located(screen: Screen, document: &str) -> Location {
    Location {
        screen,
        document: document.into(),
        ..Location::default()
    }
}

pub(super) fn specimen(record: &mut Recorder, variant: Variant) {
    let variant = Variant {
        frame: Frame {
            height: 2200,
            ..variant.frame
        },
        ..variant
    };
    specimen_gallery(record, variant);
}
