//! Container allocation and retained fields are separate from rendered design acceptance.
use super::*;
use crate::{
    keyed::keyed,
    map::Workspace,
    tests::support::{self, mounted::Mounted},
};
use recite_config::WriterPresentationField;
use tokio::sync::mpsc;
use xilem::{
    WidgetView,
    masonry::{
        core::{PointerButton, WindowEvent},
        kurbo::{Point, Rect},
        widgets::{Passthrough, SizedBox, TextArea},
    },
};
#[path = "preview/tests.rs"]
mod preview;

fn page(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    keyed((), view::app(app))
}

fn painted_menu(mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>) -> Rect {
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if widget.downcast::<SizedBox>().is_some()
            && widget.children().iter().any(|child| {
                support::native_label(&mounted.harness, child.id()).as_deref()
                    == Some("Document actions menu")
            })
        {
            return widget
                .ctx()
                .window_transform()
                .transform_rect_bbox(widget.ctx().border_box());
        }
        pending.extend(widget.children());
    }
    panic!("painted document action panel");
}

#[test]
fn allocated_window_bounds_control_reflow_and_keep_the_manuscript_across_panes() {
    for scale in [100, 200] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        writer.show_workspace(Workspace::Map);
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
        writer.map.workspace = Workspace::Script;
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config.clone());
        let mut app = App::with_writer(writer);
        app.preferences.config = config;
        app.theme.refresh(&app.preferences.config.writer);
        let properties = app.theme.properties.clone();
        let mut mounted = Mounted::new(app, page);
        mounted.harness.set_default_properties(properties);
        mounted.settle();
        let field = mounted.find("Manuscript");
        for size in [(900, 650), (1280, 720), (2040, 1230), (900, 650)] {
            mounted
                .harness
                .process_window_event(WindowEvent::Resize(size.into()));
            mounted.settle();
            assert_eq!(mounted.state.viewport.width, f64::from(size.0));
            assert_eq!(
                mounted.state.writer.as_ref().unwrap().viewport.width,
                f64::from(size.0)
            );
            for workspace in [Workspace::Map, Workspace::Split, Workspace::Script] {
                mounted.state.writer.as_mut().unwrap().map.workspace = workspace;
                mounted.settle();
                for title in ["Commands", "Workspace", "Document actions ⋯", "Copy link"] {
                    let control = mounted.harness.get_widget_with_id(mounted.find(title));
                    let bounds = control
                        .ctx()
                        .window_transform()
                        .transform_rect_bbox(control.ctx().border_box());
                    assert!(
                        bounds.x0 >= 0.
                            && bounds.x1 <= f64::from(size.0) + 0.01
                            && bounds.y1 <= f64::from(size.1) + 0.01,
                        "{title}, {workspace:?}, {size:?}, {scale}%: {bounds:?}"
                    );
                }
                assert!(
                    mounted
                        .harness
                        .get_widget_with_id(field)
                        .downcast::<TextArea<true>>()
                        .is_some()
                );
            }
            assert_eq!(mounted.find("Manuscript"), field);
        }
    }
}

#[test]
fn document_actions_overlay_preserves_the_manuscript_allocation_and_native_field() {
    for scale in [100, 200] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config.clone());
        let mut app = App::with_writer(writer);
        app.preferences.config = config;
        let mut mounted = Mounted::new(app, page);
        mounted
            .harness
            .process_window_event(WindowEvent::Resize((1280, 720).into()));
        mounted.settle();
        mounted.replace_text("Manuscript", "Keep this native draft.");
        let field = mounted.find("Manuscript");
        let before = mounted.harness.get_widget_with_id(field).ctx().border_box();
        let transform = mounted
            .harness
            .get_widget_with_id(field)
            .ctx()
            .window_transform();
        mounted.activate("Document actions ⋯");
        assert!(
            mounted
                .state
                .writer
                .as_ref()
                .unwrap()
                .document_menu
                .is_open()
        );
        assert_eq!(mounted.find("Manuscript"), field);
        assert_eq!(
            mounted.harness.get_widget_with_id(field).ctx().border_box(),
            before
        );
        assert_eq!(
            mounted
                .harness
                .get_widget_with_id(field)
                .ctx()
                .window_transform(),
            transform
        );
        let control = mounted
            .harness
            .get_widget_with_id(mounted.find("Dismiss document actions"));
        let bounds = control
            .ctx()
            .window_transform()
            .transform_rect_bbox(control.ctx().border_box());
        assert!(
            bounds.x0 >= 0. && bounds.x1 <= 1280. && bounds.y0 >= 0. && bounds.y1 <= 720.,
            "{scale}%: {bounds:?}"
        );
        let panel = painted_menu(&mounted);
        assert!(
            panel.x0 >= 0.
                && panel.x1 <= 1280.
                && panel.y0 >= 0.
                && panel.y1 <= 720.
                && panel.width() < 1280.,
            "{scale}% painted panel: {panel:?}"
        );
        let outside = mounted
            .harness
            .get_widget_with_id(mounted.find(if scale == 100 {
                "Find scene or beat…"
            } else {
                "Go to scene"
            }));
        let bounds = outside
            .ctx()
            .window_transform()
            .transform_rect_bbox(outside.ctx().border_box());
        let point = Point::new(bounds.x0 + 2., bounds.center().y);
        assert!(
            !panel.contains(point),
            "outside action {point:?}: {panel:?}"
        );
        mounted.harness.mouse_move(point);
        mounted
            .harness
            .mouse_button_press(Some(PointerButton::Primary));
        mounted
            .harness
            .mouse_button_release(Some(PointerButton::Primary));
        mounted.settle();
        assert!(
            mounted.state.picker.is_some(),
            "outside action must receive the pointer"
        );
        mounted.state.picker = None;
        mounted.settle();
        mounted.activate("Dismiss document actions");
        assert_eq!(mounted.find("Manuscript"), field);
        assert_eq!(
            mounted
                .state
                .writer
                .as_ref()
                .unwrap()
                .session
                .model()
                .draft(),
            "Keep this native draft."
        );
        for width in [900, 1280] {
            mounted
                .harness
                .process_window_event(WindowEvent::Resize((width, 720).into()));
            mounted.settle();
            mounted.activate("Document actions ⋯");
            for label in ["Save file", "Save all", "Focus writing", "Close"] {
                let control = mounted.harness.get_widget_with_id(mounted.find(label));
                let bounds = control
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(control.ctx().border_box());
                assert!(
                    bounds.x0 >= 0.
                        && bounds.x1 <= f64::from(width)
                        && bounds.y0 >= 0.
                        && bounds.y1 <= 720.,
                    "{scale}% {width}px {label}: {bounds:?}"
                );
            }
            mounted.activate("Dismiss document actions");
        }
    }
}

#[test]
fn pinned_reference_keeps_the_window_and_its_actions_within_allocated_bounds() {
    for scale in [100, 200] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.publish_reference().unwrap();
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config.clone());
        let mut app = App::with_writer(writer);
        app.preferences.config = config;
        let mut mounted = Mounted::new(app, page);
        mounted
            .harness
            .process_window_event(WindowEvent::Resize((1280, 720).into()));
        mounted.settle();
        assert_eq!(mounted.state.viewport.width, 1280.);
        assert_eq!(mounted.state.writer.as_ref().unwrap().viewport.width, 1280.);
        for label in ["Hide reference", "Unpin reference", "Try scene"] {
            let control = mounted.harness.get_widget_with_id(mounted.find(label));
            let bounds = control
                .ctx()
                .window_transform()
                .transform_rect_bbox(control.ctx().border_box());
            assert!(
                bounds.x0 >= 0. && bounds.x1 <= 1280. && bounds.y0 >= 0. && bounds.y1 <= 720.,
                "{scale}% {label}: {bounds:?}"
            );
        }
        assert!(
            mounted.contains_text("Would you tell me, please, which way I ought to go from here?")
        );
        mounted.activate("Unpin reference");
        assert!(
            mounted
                .state
                .writer
                .as_ref()
                .unwrap()
                .reading_context
                .pinned
                .is_none()
        );
    }
}
