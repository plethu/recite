use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use recite_config::WriterPresentationField;
use xilem::{
    WidgetView,
    masonry::{core::WindowEvent, widgets::Passthrough},
};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn preview_submission_retains_identity_through_preparation_cancellation_and_restart() {
    let directory = project(SOURCE).unwrap();
    for scale in [100, 200] {
        let mut writer = open(&directory).unwrap();
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config);
        writer.configure_trial();
        let (sender, mut requests) = async_mpsc::unbounded_channel();
        let mut mounted = Mounted::new(writer, surface);
        mounted.state.trial.sender = Some(sender);
        let submit = mounted.find("Start preview");
        assert!(mounted.contains_text("Inputs for the next run"));
        mounted.activate("Start preview");
        let request = requests.try_recv().unwrap();
        assert_eq!(mounted.find("Preparing preview…"), submit);
        assert!(
            mounted
                .harness
                .get_widget_with_id(submit)
                .ctx()
                .is_disabled()
        );
        mounted.activate("Cancel preparation");
        assert_eq!(mounted.find("Cancelling preparation…"), submit);
        assert!(
            mounted
                .harness
                .get_widget_with_id(mounted.find("Cancel preparation"))
                .ctx()
                .is_disabled()
        );
        assert!(matches!(
            finish(&mut mounted.state, request).unwrap(),
            Some(Retired::Prepared(_))
        ));
        mounted.settle();
        assert_eq!(mounted.find("Start preview"), submit);
        mounted.activate("Start preview");
        assert!(
            finish(&mut mounted.state, requests.try_recv().unwrap())
                .unwrap()
                .is_none()
        );
        mounted.settle();
        assert_eq!(mounted.find("Restart preview"), submit);
        assert!(mounted.contains_text("Hello."));
        for width in [1280, 900] {
            mounted.state.viewport.width = f64::from(width);
            mounted
                .harness
                .process_window_event(WindowEvent::Resize((width, 650).into()));
            for configuring in [true, false] {
                mounted.state.trial.settings.open = configuring;
                mounted.settle();
                let widget = mounted.harness.get_widget_with_id(submit);
                let bounds = widget
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(widget.ctx().border_box());
                assert!(
                    bounds.x0 >= 0.
                        && bounds.x1 <= f64::from(width)
                        && bounds.y0 >= 0.
                        && bounds.y1 <= 650.,
                    "{scale}% {width} configuring={configuring}: {bounds:?}"
                );
            }
        }
        mounted.activate("Restart preview");
        assert!(matches!(
            finish(&mut mounted.state, requests.try_recv().unwrap()).unwrap(),
            Some(Retired::Preview(_))
        ));
        mounted.settle();
        assert_eq!(mounted.find("Restart preview"), submit);
    }
}
