//! The checked-in wordmark is the same authored asset in both native frontends.
use super::{Theme, label};
use recite_config::{WriterConfig, WriterTheme};
use recite_writer_session::palette::{Palette, Rgba};
use std::sync::Arc;
use xilem::{
    AnyWidgetView, WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{flex_col, sized_box, svg},
};

pub(super) fn load(config: &WriterConfig) -> Result<Arc<usvg::Tree>, String> {
    inked(
        config,
        include_str!("../../../../../../assets/identity/recite-wordmark.svg"),
    )
}

/// The print wordmark without paper, for the welcome screen's grained desk.
pub(super) fn load_hero(config: &WriterConfig) -> Result<Arc<usvg::Tree>, String> {
    inked(
        config,
        include_str!("../../../../../../assets/identity/recite-wordmark-hero.svg"),
    )
}

/// An identity asset in this appearance's inks: plum letters and the second-ink plate.
fn inked(config: &WriterConfig, source: &str) -> Result<Arc<usvg::Tree>, String> {
    let palette = Palette::with_mode(config.theme == WriterTheme::Dark, config.monochrome);
    let source = source
        .replace("#683e62", &hex(palette.wordmark))
        .replace("#00838a", &hex(palette.teal));
    usvg::Tree::from_str(&source, &usvg::Options::default())
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn hex(color: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

/// The hero wordmark at `width`; its aspect ratio is 3.251.
pub(crate) fn hero<S: 'static>(theme: &Theme, width: f64) -> Box<AnyWidgetView<S>> {
    match &theme.hero {
        Ok(tree) => sized_box(svg(tree.clone()).with_alt_text("Recite"))
            .dims((width.px(), (width / 3.251).px()))
            .boxed(),
        Err(_) => sized_box(flex_col(())).dims((0.px(), 0.px())).boxed(),
    }
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
