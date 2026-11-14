//! Rendered gallery of every Writer surface for design review and regression comparison.
//!
//! Each scene mounts the production view with the native window's theme, bundled fonts and live
//! workers, then rasterises it with Vello CPU. No compositor or GPU is involved, so the same source
//! renders the same pixels on any host. Run it with `just writer gallery`; see
//! `apps/writer/design/README.md` for the outputs and the review workflow that consumes them.
mod compare;
pub(crate) mod record;
mod scenes;
pub(crate) mod stage;
#[cfg(test)]
mod tests;

use recite_config::{WriterConfig, WriterPresentationField, WriterTheme};
use stage::Frame;
use std::path::PathBuf;

/// One appearance a scene is rendered in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Variant {
    pub id: &'static str,
    pub theme: WriterTheme,
    pub monochrome: bool,
    pub ui_scale: u16,
    pub frame: Frame,
}
impl Variant {
    pub const LIGHT: Self = Self {
        id: "light",
        theme: WriterTheme::Light,
        monochrome: false,
        ui_scale: 100,
        frame: Frame::DEFAULT,
    };
    pub const DARK: Self = Self {
        id: "dark",
        theme: WriterTheme::Dark,
        ..Self::LIGHT
    };
    pub const MONOCHROME: Self = Self {
        id: "mono",
        monochrome: true,
        ..Self::LIGHT
    };
    pub const MINIMUM: Self = Self {
        id: "min",
        frame: Frame::MINIMUM,
        ..Self::LIGHT
    };
    pub const LARGE: Self = Self {
        id: "ui200",
        ui_scale: 200,
        ..Self::LIGHT
    };
    /// Every scene in light and dark; layout-sensitive scenes also at the extremes.
    pub const ALL: [Self; 5] = [
        Self::LIGHT,
        Self::DARK,
        Self::MONOCHROME,
        Self::MINIMUM,
        Self::LARGE,
    ];
    pub fn configure(self, config: &mut WriterConfig) {
        config.theme = self.theme;
        config.monochrome = self.monochrome;
        if let Ok(presentation) = config
            .presentation
            .with_value(WriterPresentationField::UiScale, self.ui_scale)
        {
            config.presentation = presentation;
        }
    }
}

/// Capture settings read from the environment.
pub(crate) struct Settings {
    pub output: PathBuf,
    pub baseline: Option<PathBuf>,
    pub only: Vec<String>,
    pub scale: f64,
    /// Extra frames, in milliseconds after each transition step, for reviewing motion.
    pub motion: Vec<u64>,
    /// Fail when the comparison finds an accessibility regression.
    pub strict: bool,
}
impl Settings {
    pub fn from_env() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let var = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        let list = |name: &str| -> Vec<String> {
            var(name)
                .map(|value| {
                    value
                        .split(',')
                        .map(|item| item.trim().to_owned())
                        .collect()
                })
                .unwrap_or_default()
        };
        let baseline = var("RECITE_WRITER_GALLERY_BASELINE")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("design/baseline"));
        Self {
            output: var("RECITE_WRITER_GALLERY")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("target/writer-gallery")),
            baseline: (baseline.join("manifest.json").exists()
                && var("RECITE_WRITER_GALLERY_BASELINE").as_deref() != Some("none"))
            .then_some(baseline),
            only: list("RECITE_WRITER_GALLERY_ONLY"),
            scale: var("RECITE_WRITER_GALLERY_SCALE")
                .and_then(|scale| scale.parse().ok())
                .unwrap_or(1.),
            motion: list("RECITE_WRITER_GALLERY_MOTION")
                .iter()
                .filter_map(|ms| ms.parse().ok())
                .collect(),
            strict: var("RECITE_WRITER_GALLERY_STRICT").is_some(),
        }
    }
}
