//! Logical pixels; graph world geometry is deliberately owned by the map.
use freya::{
    animation::{AnimNum, Ease, Function},
    prelude::*,
};

pub const SPACE_XS: f32 = 4.;
pub const SPACE_SM: f32 = 8.;
pub const SPACE_MD: f32 = 12.;
pub const SPACE_LG: f32 = 16.;
pub const SPACE_XL: f32 = 24.;
pub const TEXT_SMALL: f32 = 12.;
pub const TEXT_BODY: f32 = 14.;
pub const TEXT_HEADING: f32 = 18.;
pub const TEXT_PROSE: f32 = 20.;
pub const TEXT_CODE: f32 = 15.;
pub const TEXT_TITLE: f32 = 22.;
pub const SPLITTER_WIDTH: f32 = 6.;
pub const COLLAPSED_DRAWER_WIDTH: f32 = 40.;
pub const RESIZE_STEP: f32 = 16.;
pub const CONTROL_HEIGHT: f32 = 32.;
pub const RADIUS: f32 = 4.;
pub const DIALOG_RADIUS: f32 = 8.;
pub const FOCUS_WIDTH: f32 = 2.;
pub const MOTION_MS: u64 = 160;

pub fn transition(from: f32, to: f32, reduced: bool) -> AnimNum {
    AnimNum::new(from, to)
        .time(if reduced { 0 } else { MOTION_MS })
        .ease(Ease::Out)
        .function(Function::Cubic)
}

pub struct Colors {
    pub rule: Color,
    pub ink: Color,
    pub muted: Color,
    pub accent: Color,
    pub surface: Color,
    pub hover: Color,
    pub pressed: Color,
    pub on_accent: Color,
}
pub fn colors() -> Colors {
    let theme = use_theme();
    let theme = theme.read();
    let c = &theme.colors;
    Colors {
        rule: c.border,
        ink: c.text_primary,
        muted: c.text_secondary,
        accent: c.primary,
        surface: c.surface_tertiary,
        hover: c.focus,
        pressed: c.surface_primary,
        on_accent: c.background,
    }
}

/// Two short, exclusive setting labels.
pub const OPTION_GROUP_WIDTH: f32 = 200.;

/// Reserve space for passage actions so focusing prose never shifts its text.
pub const PROSE_META_HEIGHT: f32 = 28.;
