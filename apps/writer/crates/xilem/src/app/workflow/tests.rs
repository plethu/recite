use super::*;
use crate::{
    keyed::keyed,
    map::Workspace,
    tests::support::{self, mounted::Mounted},
    workspaces::Screen,
};
use recite_config::WriterCommand;
use recite_writer_session::commands;
use std::fs;
use tokio::sync::mpsc as async_mpsc;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    keyed((), view::app(app))
}
fn command<V: WidgetView<App, Widget = Passthrough>>(
    mounted: &mut Mounted<App, V>,
    command: WriterCommand,
) {
    let focused = mounted
        .state
        .writer
        .as_ref()
        .unwrap()
        .attention
        .is_focused();
    let caption = commands::label(command, focused);
    mounted.activate("Commands");
    mounted.replace_text("Search query", &caption);
    mounted.activate(&format!("Selected · {caption}"));
    assert!(mounted.state.picker.is_none(), "{caption} must execute");
}

#[test]
fn mounted_commands_compose_authoring_history_workspaces_and_explicit_save() {
    let original = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";
    let directory = support::project(original).unwrap();
    fs::write(directory.path().join("recite.project.toml"), "format_version=1\n[project]\nschema='schema.json'\n[[scenes]]\nid='start'\nasset='build/start.recitec'\nblock='start'\nparticipants=['mara']\n").unwrap();
    fs::write(
        directory.path().join("schema.json"),
        "{\"schema_version\":1,\"speakers\":{\"mara\":{\"display_name\":\"Mara\"}}}",
    )
    .unwrap();
    let app = App::with_writer(support::open(&directory).unwrap());
    let mut mounted = Mounted::new(app, surface);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    let writer = mounted.state.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    let (sender, mut saves) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    mounted.replace_text("Manuscript", "Accepted through commands.");
    command(&mut mounted, WriterCommand::Apply);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    let accepted = mounted
        .state
        .writer
        .as_ref()
        .unwrap()
        .session
        .model()
        .document()
        .source_snapshot();
    assert!(accepted.contains("Accepted through commands."));
    command(&mut mounted, WriterCommand::Undo);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        original
    );
    command(&mut mounted, WriterCommand::Redo);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        accepted.as_ref()
    );
    command(&mut mounted, WriterCommand::Split);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    assert_eq!(
        mounted.state.writer.as_ref().unwrap().map.workspace,
        Workspace::Split
    );
    command(&mut mounted, WriterCommand::Map);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    assert_eq!(
        mounted.state.writer.as_ref().unwrap().map.workspace,
        Workspace::Map
    );
    command(&mut mounted, WriterCommand::Script);
    mounted.settle();
    command(&mut mounted, WriterCommand::AddBeat);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    command(&mut mounted, WriterCommand::AddLine);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    command(&mut mounted, WriterCommand::AddReply);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    mounted.settle();
    let structured = mounted
        .state
        .writer
        .as_ref()
        .unwrap()
        .session
        .model()
        .document()
        .source_snapshot();
    assert_eq!(
        structured
            .lines()
            .filter(|line| line.starts_with(":: "))
            .count(),
        2
    );
    assert_eq!(
        structured
            .lines()
            .filter(|line| line.trim_start().starts_with("> "))
            .count(),
        3 // Add beat includes its first line; Add line contributes another.
    );
    assert_eq!(
        structured
            .lines()
            .filter(|line| line.trim_start().starts_with("? "))
            .count(),
        1
    );
    assert!(structured.contains("line@11111111111111111111"));
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        original
    );
    command(&mut mounted, WriterCommand::SaveAll);
    support::finish_work(mounted.state.writer.as_mut().unwrap(), &mut saves).unwrap();
    mounted.settle();
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        structured.as_ref()
    );
    for (command_id, screen) in [
        (WriterCommand::Declarations, Screen::Declarations),
        (WriterCommand::Build, Screen::Build),
        (WriterCommand::Rename, Screen::Rename),
    ] {
        command(&mut mounted, command_id);
        support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation)
            .unwrap();
        mounted.settle();
        assert!(mounted.state.writer.as_ref().unwrap().workspaces.screen == screen);
        mounted.state.writer.as_mut().unwrap().return_to_writing();
        mounted.settle();
    }
    mounted.activate("Projects");
    assert!(mounted.state.show_welcome);
    mounted.activate("Create a new project");
    mounted.type_text(
        "Project folder",
        directory.path().join("other-project").to_str().unwrap(),
    );
    mounted.type_text("Project name", "A new project");
    mounted.activate("Return to Writer");
    assert!(!mounted.state.show_welcome);
    assert!(mounted.state.keep_running());
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        structured.as_ref()
    );
}

#[test]
fn mounted_project_text_pages_load_all_matches_and_apply_the_field_only_on_navigation() {
    let source = (0..130)
        .map(|index| {
            format!(
                ":: beat{index:03}{}\n> line@{index:020x}\n  Shared needle{index:03}.\n-> END\n",
                if index == 0 { " default" } else { "" }
            )
        })
        .collect::<String>();
    let directory = support::project(&source).unwrap();
    let app = App::with_writer(support::open(&directory).unwrap());
    let mut mounted = Mounted::new(app, surface);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    let writer = mounted.state.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    mounted.replace_text("Manuscript", "Typing before lookup.");
    mounted.activate("Find project text");
    mounted.type_text("Search query", "Shared");
    mounted.activate("Next results");
    mounted.activate("Previous results");
    mounted.activate("Load more matches");
    assert!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .has_draft()
    );
    assert!(!mounted.state.writer.as_ref().unwrap().navigation.busy());
    mounted.replace_text("Search query", "needle129");
    mounted.activate_prefix("Selected · scene.recite · ");
    assert!(mounted.state.picker.is_none());
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut navigation).unwrap();
    let model = mounted.state.writer.as_ref().unwrap().session.model();
    assert_eq!(model.selected_block().unwrap().as_deref(), Some("beat129"));
    assert!(!model.has_draft());
    assert!(model.document().source().contains("Typing before lookup."));
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
}
