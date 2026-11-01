use super::*;
use crate::workspaces::{Screen, tests::ToolDriver};
use recite_writer_session::typography::BUNDLED_FONTS;
use std::{fs, sync::Arc};
use xilem::masonry::{
    accesskit::{NodeId, TreeUpdate},
    app::{RenderRoot, RenderRootOptions, WindowSizePolicy},
    peniko::Blob,
    widgets::Button,
};
use xilem::{ViewCtx, core::View as _};

fn render(app: &mut App, width: u32) -> (RenderRoot, TreeUpdate) {
    app.viewport.width = f64::from(width);
    app.viewport.height = 720.;
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = view::app(app);
    let (pod, _) = view.build(&mut context, app);
    let mut root = RenderRoot::new(
        pod.new_widget,
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(app.theme.initial_properties()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: (width, 720).into(),
            scale_factor: 1.,
            test_font: None,
        },
    );
    // Use the same bundled font metrics as the native application.
    for (_, bytes) in BUNDLED_FONTS {
        root.register_fonts(Blob::new(Arc::new(bytes.to_vec())));
    }
    root.handle_window_event(WindowEvent::EnableAccessTree);
    let (_, update) = root.redraw();
    (root, update.expect("initial accessible project header"))
}

fn header_button(root: &RenderRoot, update: &TreeUpdate, name: &str) -> Rect {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        if widget.downcast::<Button>().is_some()
            && widget
                .children()
                .iter()
                .any(|child| accessible_caption(update, NodeId(child.id().to_raw())) == name)
        {
            return widget
                .ctx()
                .window_transform()
                .transform_rect_bbox(widget.ctx().border_box());
        }
        pending.extend(widget.children().into_iter().rev());
    }
    panic!("missing project header button {name}");
}

fn accessible_caption(update: &TreeUpdate, id: NodeId) -> String {
    let mut pending = vec![id];
    let mut caption = String::new();
    while let Some(id) = pending.pop() {
        let (_, node) = update
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("published child");
        if let Some(text) = node.label().or(node.value()) {
            caption.push_str(text);
        } else {
            pending.extend(node.children().iter().rev());
        }
    }
    caption
}

#[test]
fn project_screen_headers_keep_complete_buttons_inside_the_allocated_window() {
    for scale in [100, 200] {
        for relative in [
            "src/relay_hub.recite",
            "src/dialogue/scenes/relay_hub_station_history_and_courier_notes.recite",
        ] {
            let directory = support::project(recite_writer_model::FIXTURE).unwrap();
            let path = directory.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, recite_writer_model::FIXTURE).unwrap();
            let mut writer = support::open(&directory).unwrap();
            writer.session.project_mut().unwrap().switch(&path).unwrap();
            writer.change(|model| {
                model.set_draft("Unsaved author text.".into());
                model.apply()?;
                Ok(())
            });
            assert!(writer.session.current_dirty());
            let mut config = writer.config.clone();
            config.writer.presentation = config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.configure(config.clone());
            let mut tools = ToolDriver::new(&mut writer);
            fs::write(
                &path,
                recite_writer_model::FIXTURE.replace("Would you tell me", "Could you tell me"),
            )
            .unwrap();
            writer.compare_disk_changes();
            tools.finish(&mut writer);
            writer.keep_writer_version();
            tools.finish(&mut writer);
            assert_eq!(
                writer.status,
                "Kept Writer's document and draft. The next Save replaces the inspected disk version."
            );
            let mut app = App::with_writer(writer);
            app.preferences.config = config;
            app.theme.refresh(&app.preferences.config.writer);
            for width in [900, 1280, 2040] {
                for screen in [Screen::External, Screen::Settings, Screen::Localisation] {
                    app.writer.as_mut().unwrap().workspaces.screen = screen;
                    let (root, update) = render(&mut app, width);
                    let left = app.writer.as_ref().unwrap().navigation_width() + 16.;
                    for title in [
                        "Back",
                        "Forward",
                        "Copy link",
                        "Return to writing",
                        "Save all",
                        "Close",
                    ] {
                        let bounds = header_button(&root, &update, title);
                        assert!(
                            bounds.x0 >= left - 0.01
                                && bounds.x1 <= f64::from(width) - 16. + 0.01
                                && bounds.y0 >= 0.
                                && bounds.y1 <= 720.,
                            "{title}, {relative}, {width}px, {scale}%: {bounds:?}"
                        );
                    }
                }
            }
            assert!(app.writer.as_ref().unwrap().session.current_dirty());
        }
    }
}
