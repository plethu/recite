use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use recite_config::WriterConfig;
use xilem::masonry::{core::WidgetId, kurbo::Rect, widgets::Passthrough};

const WIDTH: f64 = 600.;

fn view(_: &mut ()) -> impl WidgetView<(), Widget = Passthrough> + use<> {
    let palette = crate::presentation::palette(&WriterConfig::default());
    crate::keyed::keyed(
        (),
        flex_col((banner(
            &palette,
            (1., "Changed on disk"),
            (label(1., "1 file"), label(1., "Compare")),
            false,
            WIDTH,
        ),)),
    )
}

/// The window rectangle of `id`'s border box.
fn bounds(mounted: &Mounted<(), impl WidgetView<(), Widget = Passthrough>>, id: WidgetId) -> Rect {
    let widget = mounted.harness.get_widget_with_id(id);
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}

#[test]
fn a_banner_is_an_errata_slip_as_tall_as_its_words_and_padding() {
    let mut mounted = Mounted::new((), view);
    let palette = crate::presentation::palette(&WriterConfig::default());
    // The lead-in names the notice in caps; the message follows it.
    assert!(mounted.contains_text("CHANGED ON DISK"));
    let root = mounted.harness.root_widget().as_dyn();
    let slip = bounds(&mounted, root.children()[0].children()[0].id());
    let mut pending = vec![root];
    let mut message = None;
    while let Some(widget) = pending.pop() {
        if widget.inner().get_debug_text().as_deref() == Some("1 file") {
            message = Some(widget.id());
        }
        pending.extend(widget.children());
    }
    let words = bounds(&mounted, message.expect("the message"));
    // The edge and the stock come out of the padding: no taller than the words and the old
    // padding above and below them.
    assert_eq!(slip.width(), WIDTH);
    assert!(slip.height() <= words.height() + 2. * space::SIBLINGS + 1.);
    let image = mounted.harness.render();
    let rgba = |color: Color| color.to_rgba8().to_u8_array();
    let at = |x: f64, y: f64| image.get_pixel(x as u32, y as u32).0;
    // A closed boundary edge, floating paper inside it, and the hard stock at the slip's foot
    // and trailing side, with no bar down its leading side.
    assert_eq!(at(slip.x0, slip.y0 + 10.), rgba(palette.boundary));
    assert_eq!(at(slip.x0 + 4., slip.y0 + 10.), rgba(palette.floating));
    assert_eq!(at(slip.x1 - 1., slip.y0 + 10.), rgba(palette.stock));
    assert_eq!(at(slip.x0 + 20., slip.y1 - 1.), rgba(palette.stock));
}
