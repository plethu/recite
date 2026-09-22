//! Validated display preferences. No project data or viewport coordinates live here.
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "RawPresentation")]
pub struct WriterPresentation {
    reading_size: u16,
    source_size: u16,
    ui_scale: u16,
    drawer_width: u16,
    script_width: u16,
    split: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriterPresentationField {
    ReadingSize,
    SourceSize,
    UiScale,
    DrawerWidth,
    ScriptWidth,
}

impl WriterPresentationField {
    pub const fn bounds(self) -> (u16, u16) {
        match self {
            Self::ReadingSize => (14, 32),
            Self::SourceSize => (12, 28),
            Self::UiScale => (100, 200),
            Self::DrawerWidth => (180, 360),
            Self::ScriptWidth => (320, 800),
        }
    }
    pub const fn key(self) -> &'static str {
        match self {
            Self::ReadingSize => "reading_size",
            Self::SourceSize => "source_size",
            Self::UiScale => "ui_scale",
            Self::DrawerWidth => "drawer_width",
            Self::ScriptWidth => "script_width",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("writer presentation {field:?} must be between {min} and {max}, found {value}")]
pub struct WriterPresentationError {
    field: WriterPresentationField,
    min: u16,
    max: u16,
    value: u16,
}

impl Default for WriterPresentation {
    fn default() -> Self {
        Self {
            reading_size: 20,
            source_size: 15,
            ui_scale: 100,
            drawer_width: 224,
            script_width: 520,
            split: false,
        }
    }
}
impl WriterPresentation {
    pub const fn value(self, field: WriterPresentationField) -> u16 {
        match field {
            WriterPresentationField::ReadingSize => self.reading_size,
            WriterPresentationField::SourceSize => self.source_size,
            WriterPresentationField::UiScale => self.ui_scale,
            WriterPresentationField::DrawerWidth => self.drawer_width,
            WriterPresentationField::ScriptWidth => self.script_width,
        }
    }
    pub fn with_value(
        mut self,
        field: WriterPresentationField,
        value: u16,
    ) -> Result<Self, WriterPresentationError> {
        let (min, max) = field.bounds();
        if !(min..=max).contains(&value) {
            return Err(WriterPresentationError {
                field,
                min,
                max,
                value,
            });
        }
        match field {
            WriterPresentationField::ReadingSize => self.reading_size = value,
            WriterPresentationField::SourceSize => self.source_size = value,
            WriterPresentationField::UiScale => self.ui_scale = value,
            WriterPresentationField::DrawerWidth => self.drawer_width = value,
            WriterPresentationField::ScriptWidth => self.script_width = value,
        }
        Ok(self)
    }
    pub const fn split(self) -> bool {
        self.split
    }
    pub const fn with_split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawPresentation {
    reading_size: u16,
    source_size: u16,
    ui_scale: u16,
    drawer_width: u16,
    script_width: u16,
    split: bool,
}
impl Default for RawPresentation {
    fn default() -> Self {
        let v = WriterPresentation::default();
        Self {
            reading_size: v.reading_size,
            source_size: v.source_size,
            ui_scale: v.ui_scale,
            drawer_width: v.drawer_width,
            script_width: v.script_width,
            split: v.split,
        }
    }
}
impl TryFrom<RawPresentation> for WriterPresentation {
    type Error = WriterPresentationError;
    fn try_from(raw: RawPresentation) -> Result<Self, Self::Error> {
        use WriterPresentationField::*;
        let mut value = Self::default().with_split(raw.split);
        for (field, number) in [
            (ReadingSize, raw.reading_size),
            (SourceSize, raw.source_size),
            (UiScale, raw.ui_scale),
            (DrawerWidth, raw.drawer_width),
            (ScriptWidth, raw.script_width),
        ] {
            value = value.with_value(field, number)?;
        }
        Ok(value)
    }
}
