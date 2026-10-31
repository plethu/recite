//! Authored local font fallback chains shared by native Writer adapters.
mod embedded;
pub use embedded::BUNDLED_FONTS;
/// Font fallbacks stay local. Both adapters register the bundled OFL faces before
/// falling back to the system's families for scripts those faces do not cover.
pub const FONT_UI: &[&str] = &[
    "IBM Plex Sans",
    "Inter",
    "Geist",
    "Segoe UI",
    "Helvetica Neue",
    "Noto Sans",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Native font managers also fall back per glyph; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "sans-serif",
];
pub const FONT_PROSE: &[&str] = &[
    "Literata",
    "Source Serif 4",
    "Noto Serif",
    "Georgia",
    // Serif companions where the system has them; the sans families cover the rest.
    "Songti SC",
    "Hiragino Mincho ProN",
    "Noto Serif CJK SC",
    "Noto Serif CJK JP",
    "Noto Serif CJK KR",
    "Noto Naskh Arabic",
    "Noto Serif Hebrew",
    "Noto Serif Devanagari",
    "Noto Serif Thai",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Native font managers also fall back per glyph; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "serif",
];
/// Pane titles and identity lockups; falls back to the reading face.
pub const FONT_DISPLAY: &[&str] = &[
    // The static 72pt Soft cut carries the print shape; the variable font defaults to opsz 144.
    "Fraunces 72pt Soft",
    "Fraunces",
    "Literata",
    "Source Serif 4",
    "Noto Serif",
    "Georgia",
    // Serif companions where the system has them; the sans families cover the rest.
    "Songti SC",
    "Hiragino Mincho ProN",
    "Noto Serif CJK SC",
    "Noto Serif CJK JP",
    "Noto Serif CJK KR",
    "Noto Naskh Arabic",
    "Noto Serif Hebrew",
    "Noto Serif Devanagari",
    "Noto Serif Thai",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // The shaper also falls back per glyph through system fonts; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "serif",
];
/// Facts about the text rather than the text: file names, IDs, counts, shortcuts.
pub const FONT_MONO: &[&str] = &[
    "IBM Plex Mono",
    "Menlo",
    "Cascadia Mono",
    "DejaVu Sans Mono",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // The shaper also falls back per glyph through system fonts; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "monospace",
];
