//! The checked-in wordmark is the same authored asset in both native frontends.
use super::{Theme, label};
use recite_config::{WriterConfig, WriterTheme};
use recite_writer_session::palette::{Palette, Rgba};
use std::sync::Arc;
use xilem::{
    AnyWidgetView, WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{sized_box, svg},
};

pub(super) fn load(config: &WriterConfig) -> Result<Arc<usvg::Tree>, String> {
    let palette = Palette::with_mode(config.theme == WriterTheme::Dark, config.monochrome);
    let source = include_str!("../../../../../../assets/identity/recite-wordmark.svg")
        .replace("#683e62", &hex(palette.wordmark))
        .replace("#00838a", &hex(palette.teal));
    usvg::Tree::from_str(&source, &usvg::Options::default())
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn hex(color: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

pub(crate) fn wordmark<S: 'static>(theme: &Theme, scale: f32) -> Box<AnyWidgetView<S>> {
    match &theme.wordmark {
        Ok(tree) => sized_box(svg(tree.clone()).with_alt_text("Recite"))
            .dims((
                (90.1 * f64::from(scale)).px(),
                (27. * f64::from(scale)).px(),
            ))
            .boxed(),
        Err(error) => label(scale, format!("Recite wordmark: {error}")).boxed(),
    }
}
