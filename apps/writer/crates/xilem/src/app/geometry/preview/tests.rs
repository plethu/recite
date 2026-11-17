use super::*;

fn preview_bounds(mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>) -> Rect {
    labelled_bounds(mounted, "Conversation preview")
}

fn labelled_bounds(
    mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>,
    name: &str,
) -> Rect {
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if support::native_label(&mounted.harness, widget.id()).as_deref() == Some(name) {
            return widget
                .ctx()
                .window_transform()
                .transform_rect_bbox(widget.ctx().border_box());
        }
        pending.extend(widget.children());
    }
    panic!("no widget labelled {name}");
}

#[test]
fn configuring_preview_uses_the_workspace_and_restores_retained_writing_and_reference() {
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
        let field = mounted.find("Manuscript");
        assert!(mounted.contains_text("Pinned snapshot"));
        mounted.replace_text("Manuscript", "Keep pending writing.");
        mounted.activate("Workspace");
        mounted.activate("Configure preview");
        let expanded = preview_bounds(&mounted);
        assert!(expanded.height() > 320., "{scale}%: {expanded:?}");
        for width in [900, 1280] {
            mounted
                .harness
                .process_window_event(WindowEvent::Resize((width, 720).into()));
            mounted.settle();
            for name in ["Return to writing", "Done with preview inputs"] {
                let control = mounted.harness.get_widget_with_id(mounted.find(name));
                let bounds = control
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(control.ctx().border_box());
                assert!(
                    bounds.x0 >= 0. && bounds.x1 <= f64::from(width) && bounds.y1 <= 720.,
                    "{scale}% {width}px {name}: {bounds:?}"
                );
            }
        }
        assert!(mounted.harness.get_widget_with_id(field).ctx().is_stashed());
        assert!(!mounted.contains_text("Pinned snapshot"));
        mounted
            .state
            .writer
            .as_mut()
            .unwrap()
            .trial
            .settings
            .variant = "formal".into();
        mounted.activate("Return to writing");
        assert_eq!(mounted.find("Manuscript"), field);
        assert!(!mounted.harness.get_widget_with_id(field).ctx().is_stashed());
        assert!(mounted.contains_text("Pinned snapshot"));
        let writer = mounted.state.writer.as_ref().unwrap();
        assert_eq!(writer.trial.settings.variant, "formal");
        assert_eq!(writer.session.model().draft(), "Keep pending writing.");
        assert!(!writer.trial.visible);
        assert!(!writer.trial.settings.open);
        mounted.activate("Workspace");
        mounted.activate("Configure preview");
        mounted.activate("Return to writing");
        assert_eq!(mounted.find("Manuscript"), field);
        let writer = mounted.state.writer.as_ref().unwrap();
        assert!(!writer.trial.visible);
        assert!(!writer.trial.settings.open);
        assert_eq!(writer.trial.settings.variant, "formal");
        assert_eq!(writer.session.model().draft(), "Keep pending writing.");
    }
}

#[test]
fn current_run_stays_under_configuration_and_focus_retains_the_collapsed_reference() {
    let prose = "A long running conversation remains inspectable. ".repeat(80);
    let source = format!(":: start default\n> @ffee6p8\n  {prose}\n-> END\n");
    let directory = support::project(&source).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.publish_reference().unwrap();
    writer.reading_context.pinned.as_mut().unwrap().collapsed = true;
    writer.change(|model| model.start_preview());
    let mut mounted = Mounted::new(App::with_writer(writer), page);
    let manuscript = mounted.find("Manuscript");
    let reference = mounted.find("Unpin reference");
    mounted.activate("Workspace");
    mounted.activate("Configure preview");
    let bounds = |name| {
        let widget = mounted.harness.get_widget_with_id(mounted.find(name));
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
    };
    // The inputs open on a slip over the preview; the current run stays mounted beneath it.
    assert!(bounds("Continue preview").height() > 0.);
    assert!(bounds("Preview wording").height() > 0.);
    assert!(
        mounted
            .harness
            .get_widget_with_id(manuscript)
            .ctx()
            .is_stashed()
    );
    assert!(
        mounted
            .harness
            .get_widget_with_id(reference)
            .ctx()
            .is_stashed()
    );
    mounted.type_text("Preview wording", "formal");
    mounted.state.writer.as_mut().unwrap().toggle_focus();
    mounted.settle();
    assert!(
        !mounted
            .harness
            .get_widget_with_id(manuscript)
            .ctx()
            .is_stashed()
    );
    assert!(
        !mounted
            .harness
            .get_widget_with_id(reference)
            .ctx()
            .is_stashed()
    );
    assert!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .reading_context
            .pinned
            .as_ref()
            .unwrap()
            .collapsed
    );
    mounted.state.writer.as_mut().unwrap().toggle_focus();
    mounted.settle();
    assert!(
        mounted
            .harness
            .get_widget_with_id(reference)
            .ctx()
            .is_stashed()
    );
    mounted.activate("Return to writing");
    assert_eq!(mounted.find("Manuscript"), manuscript);
    let writer = mounted.state.writer.as_ref().unwrap();
    assert_eq!(writer.trial.settings.variant, "formal");
    assert_eq!(writer.session.model().preview_page().unwrap().text, prose);
}

fn named_bounds(
    mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>,
    name: &str,
) -> Rect {
    let widget = mounted.harness.get_widget_with_id(mounted.find(name));
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}

#[test]
fn preview_is_a_sheet_with_the_run_s_actions_in_its_head_and_speakers_in_the_margin() {
    let source = ":: start default\n> speaker=mara @ffee6p8\n  Hello there.\n> speaker=mara @rthh7ab\n  Still here.\n-> END\n";
    let directory = support::project(source).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| model.start_preview());
    writer.trial.visible = true;
    let mut mounted = Mounted::new(App::with_writer(writer), page);
    mounted
        .harness
        .process_window_event(WindowEvent::Resize((1280, 720).into()));
    mounted.settle();
    let pane = preview_bounds(&mounted);
    let paper = labelled_bounds(&mounted, "Transcript");
    // The paper rests on the desk, narrower than the pane.
    assert!(paper.width() + 16. <= pane.width(), "{paper:?} in {pane:?}");
    // Continue and the close icon sit in the head, above the transcript, the close at its right.
    let next = named_bounds(&mounted, "Continue preview");
    let close = named_bounds(&mounted, "Return to writing");
    assert!(
        next.y1 <= paper.y0 && close.y1 <= paper.y0,
        "{next:?} {close:?} {paper:?}"
    );
    assert!(close.x0 > next.x1, "{close:?} after {next:?}");
    assert!(mounted.contains_text("MARA"));
    mounted.activate("Continue preview");
    // The shown line joins the transcript with its speaker; the next line names its own.
    assert!(mounted.contains_text("Hello there."));
    assert!(mounted.contains_text("Still here."));
    assert!(mounted.contains_text("MARA"));
}
