//! At 200% in a 1200×800 window every overlay stays inside the window; its content reflows or
//! scrolls within it.
use super::*;
use crate::Writer;
use crate::app::picker::Search;
use crate::native_host::keyboard::focus::slip_anchor;
use crate::workspaces::tests::ToolDriver;

/// Opens one overlay over the window.
type Open = fn(&mut App);

fn writer_of(
    mounted: &mut Mounted<App, impl WidgetView<App, Widget = Passthrough>>,
) -> &mut Writer {
    mounted.state.writer.as_mut().unwrap()
}

fn dialogs(
    mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>,
) -> Vec<(String, Rect)> {
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut found = Vec::new();
    while let Some(node) = pending.pop() {
        if node.role() == Role::Dialog
            && let Some(bounds) = node.bounding_box()
        {
            let bounds = Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1);
            found.push((node.label().unwrap_or_default(), bounds));
        }
        pending.extend(node.children());
    }
    found
}

#[test]
fn no_overlay_runs_past_a_1200_by_800_window_at_200_percent() {
    let overlays: [(&str, Open); 7] = [
        ("Settings", |app| app.settings_from_menu()),
        ("Commands", |app| app.open_picker(Search::Commands)),
        ("find", |app| app.open_picker(Search::ProjectText)),
        ("go to", |app| app.open_picker(Search::GoTo)),
        ("close decision", |app| {
            let writer = app.writer.as_mut().unwrap();
            writer.change(|model| {
                model.set_draft("Unsaved line.".into());
                Ok(())
            });
            app.request_close();
        }),
        ("rename slip", |app| {
            let writer = app.writer.as_mut().unwrap();
            let mut tools = ToolDriver::new(writer);
            writer.rename_beat();
            writer.edit_inline_rename("renamed".into());
            writer.commit_inline_rename();
            tools.finish(writer);
        }),
        ("configure slip", |app| {
            let writer = app.writer.as_mut().unwrap();
            writer.trial.visible = true;
            writer.configure_trial();
        }),
    ];
    for (overlay, open) in overlays {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, 200)
            .unwrap();
        writer.configure(config.clone());
        let mut app = App::with_writer(writer);
        app.preferences.config = config;
        app.theme.refresh(&app.shell.preferences.config.writer);
        let properties = app.theme.properties.clone();
        let mut mounted = Mounted::new(app, page);
        mounted.harness.set_default_properties(properties);
        mounted
            .harness
            .process_window_event(WindowEvent::Resize((1200, 800).into()));
        mounted.settle();
        open(&mut mounted.state);
        mounted.settle();
        if overlay == "rename slip" {
            // The native host measures the heading and hangs the slip from it.
            let heading =
                slip_anchor(mounted.harness.render_root()).expect("a heading to hang from");
            writer_of(&mut mounted).place_inline_rename(heading.0, heading.1);
            mounted.settle();
        }
        let found = dialogs(&mounted);
        assert!(!found.is_empty(), "the {overlay} is a dialog");
        for (name, bounds) in found {
            assert!(
                bounds.x0 >= 0. && bounds.x1 <= 1200. && bounds.y0 >= 0. && bounds.y1 <= 800.,
                "the {overlay} ({name}) runs past the window: {bounds:?}"
            );
        }
    }
}
