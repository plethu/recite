#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::opening::Job;
use crate::app::picker;
use crate::app::view;
use crate::session::Session;
use crate::tests::support::mounted::Mounted;
use recite_config::WriterCommand;
use recite_config::WriterPresentationField;
use recite_writer_app::shell::welcome::{FormScope, ProjectAction};
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use xilem::WidgetView;
use xilem::masonry::{core::WindowEvent, widgets::Passthrough};

fn page(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), view::app(app))
}

#[test]
fn project_actions_stay_visible_at_both_scales_without_replacing_folder_fields() {
    for scale in [100, 200] {
        let mut app = App::empty();
        app.preferences.config.writer.presentation = app
            .preferences
            .config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        app.theme.refresh(&app.shell.preferences.config.writer);
        let properties = app.theme.properties.clone();
        let mut mounted = Mounted::new(app, page);
        mounted.harness.set_default_properties(properties);
        let folder = mounted.find("Project folder");
        for size in [(1280, 720), (900, 650)] {
            mounted
                .harness
                .process_window_event(WindowEvent::Resize(size.into()));
            mounted.settle();
            for creating in [false, true, false] {
                mounted.state.creating = creating;
                mounted.state.status = if creating {
                    format!(
                        "Could not open the project: {}",
                        "a long folder name / ".repeat(80)
                    )
                } else {
                    String::new()
                };
                mounted.settle();
                assert_eq!(mounted.find("Project folder"), folder);
                for name in [
                    if creating {
                        "Create project"
                    } else {
                        "Open selected project"
                    },
                    "Try temporary examples",
                ] {
                    let control = mounted.harness.get_widget_with_id(mounted.find(name));
                    let bounds = control
                        .ctx()
                        .window_transform()
                        .transform_rect_bbox(control.ctx().border_box());
                    assert!(
                        bounds.x0 >= 0.
                            && bounds.x1 <= f64::from(size.0)
                            && bounds.y0 >= 0.
                            && bounds.y1 <= f64::from(size.1),
                        "{scale}% {size:?} {name}: {bounds:?}"
                    );
                    if scale == 100
                        && size.0 == 1280
                        && !creating
                        && name == "Open selected project"
                    {
                        let field = mounted.harness.get_widget_with_id(folder);
                        let field_bounds = field
                            .ctx()
                            .window_transform()
                            .transform_rect_bbox(field.ctx().border_box());
                        assert!(
                            bounds.y0 - field_bounds.y1 <= 80.,
                            "short form actions stay beside their inputs: {field_bounds:?} {bounds:?}"
                        );
                    }
                }
                if let Ok(prefix) = std::env::var("RECITE_UX_CAPTURE") {
                    mounted
                        .harness
                        .render()
                        .save(format!(
                            "{prefix}-form-{scale}-{}-{}.png",
                            size.0,
                            if creating { "create-error" } else { "open" }
                        ))
                        .unwrap();
                }
            }
        }
    }
}

#[test]
fn project_submission_guards_empty_fields_busy_work_and_native_path_identity() {
    let mut app = App::empty();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    app.sender = Some(sender);
    app.open_selected();
    assert!(jobs.try_recv().is_err());
    app.path = " /tmp/new-project ".into();
    app.creating = true;
    app.name = "  ".into();
    app.open_selected();
    assert!(jobs.try_recv().is_err());
    app.name = "Station".into();
    app.auxiliary = true;
    app.open_selected();
    assert!(jobs.try_recv().is_err());
    app.auxiliary = false;
    app.native_path = Some(PathBuf::from("/tmp/native-project"));
    app.open_selected();
    let Job::Create { path, name } = jobs.try_recv().unwrap() else {
        panic!("one guarded create request");
    };
    assert_eq!(path, PathBuf::from("/tmp/native-project"));
    assert_eq!(name, "Station");
    app.open_selected();
    assert!(jobs.try_recv().is_err(), "one outstanding creation");
}

#[test]
fn queued_project_submission_cannot_inherit_changed_form_inputs() {
    let mut app = App::empty();
    app.path = "/tmp/project".into();
    let initial = FormScope::capture(&app);
    app.path.push('2');
    assert!(initial != FormScope::capture(&app));
    app.path.pop();
    app.creating = true;
    assert!(initial != FormScope::capture(&app));
    app.creating = false;
    app.native_path = Some(PathBuf::from("/tmp/another-project"));
    assert!(initial != FormScope::capture(&app));
    app.native_path = None;
    app.auxiliary = true;
    assert!(initial != FormScope::capture(&app));
}

#[test]
fn project_failure_leads_with_recovery_and_discloses_the_original_diagnostic() {
    let mut app = App::empty();
    app.path = "/tmp/missing-project".into();
    app.project_failed(
        ProjectAction::Open,
        "Cannot read /tmp/missing-project/recite.project.toml".into(),
    );
    let mut mounted = Mounted::new(app, page);
    let folder = mounted.find("Project folder");
    assert!(mounted.contains_text("Project didn’t open"));
    assert!(mounted.contains_text("Choose the folder containing recite.project.toml"));
    assert!(!mounted.contains_text("Cannot read /tmp/missing-project"));
    mounted.activate("Show project error details");
    assert!(mounted.contains_text("Cannot read /tmp/missing-project"));
    mounted.replace_text("Project folder", "/tmp/corrected-project");
    assert_eq!(mounted.find("Project folder"), folder);
    assert!(mounted.state.welcome_problem.is_none());
    assert!(mounted.state.status.is_empty());
}

#[test]
fn creating_a_project_shows_its_name_field_above_the_actions() {
    let mut mounted = Mounted::new(App::empty(), page);
    mounted
        .harness
        .process_window_event(WindowEvent::Resize((1200, 800).into()));
    mounted.settle();
    mounted.activate("Create a new project");
    mounted.settle();
    let bounds = |name: &str| {
        let control = mounted.harness.get_widget_with_id(mounted.find(name));
        control
            .ctx()
            .window_transform()
            .transform_rect_bbox(control.ctx().border_box())
    };
    let field = bounds("Project name");
    let action = bounds("Create project");
    assert!(
        field.y1 <= action.y0,
        "the name field must not sit under the actions: {field:?} {action:?}"
    );
}
