use super::*;
use crate::{Writer, keyed::keyed, tests::support::mounted::Mounted};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

fn relay_hub(beat: Option<&str>) -> Writer {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer
        .follow_location(&recite_writer_session::location::Location {
            document: "relay_hub.recite".into(),
            beat: beat.map(Into::into),
            ..Default::default()
        })
        .unwrap();
    writer.viewport.width = 1200.;
    writer.viewport.height = 800.;
    writer
}

fn viewport(mounted: &mut Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>) -> f64 {
    let root = mounted.harness.render_root();
    let portal = reading::manuscript(root, None).unwrap().id();
    root.get_widget(portal)
        .unwrap()
        .downcast::<Portal<Flex>>()
        .unwrap()
        .get_viewport_pos()
        .y
}

fn head_shown(
    mounted: &mut Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
) -> bool {
    let root = mounted.harness.render_root();
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if let Some(head) = widget.downcast::<RunningHead>() {
            return head.inner().is_shown();
        }
        pending.extend(widget.children());
    }
    panic!("no running head");
}

#[test]
fn a_chosen_later_beat_comes_to_the_top_and_the_running_head_follows_the_scroll() {
    let mut mounted = Mounted::new(relay_hub(None), surface);
    let mut galley = Galley::default();
    galley.synchronize(mounted.harness.render_root());
    assert_eq!(
        viewport(&mut mounted),
        0.,
        "the first beat keeps the scene title"
    );
    assert!(!head_shown(&mut mounted));
    let mut mounted = Mounted::new(relay_hub(Some("station_history")), surface);
    let mut galley = Galley::default();
    galley.synchronize(mounted.harness.render_root());
    let scrolled = viewport(&mut mounted);
    assert!(
        scrolled > TITLE,
        "the chosen beat's heading leads the sheet: {scrolled}"
    );
    assert!(
        head_shown(&mut mounted),
        "the scene title has left the sheet"
    );
    let root = mounted.harness.render_root();
    let portal = reading::manuscript(root, None).unwrap();
    let heading = heading_offset(&portal).unwrap().0 - scrolled;
    assert_eq!(
        heading,
        HEAD + LEAD,
        "the heading sits clear of the running head"
    );
}

#[test]
fn the_first_beat_keeps_the_scene_title_whole_at_200_percent() {
    let mut writer = relay_hub(Some("relay_desk"));
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let mut mounted = Mounted::new(writer, surface);
    let mut galley = Galley::default();
    galley.synchronize(mounted.harness.render_root());
    assert_eq!(viewport(&mut mounted), 0., "the scene title stays whole");
}
