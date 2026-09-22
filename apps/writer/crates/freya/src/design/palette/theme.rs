//! Freya's stock components consume the same palette as the writer components.
use super::{Palette, syntax};
use crate::design::tokens as t;
use freya::{code_editor::*, prelude::*};

pub fn theme(dark: bool) -> Theme {
    let p = Palette::new(dark);
    let mut theme = if dark { dark_theme() } else { light_theme() };
    let c = &mut theme.colors;
    c.primary = p.accent;
    c.background = p.canvas;
    c.surface_primary = p.inset;
    c.surface_secondary = p.hover;
    c.surface_tertiary = p.surface;
    c.text_primary = p.ink;
    c.text_secondary = p.muted;
    c.text_placeholder = p.muted;
    c.text_inverse = p.on_accent;
    c.text_highlight = p.selection;
    c.border = p.boundary;
    c.border_focus = p.accent;
    c.border_disabled = p.rule;
    c.focus = p.selection;
    c.active = p.pressed;
    c.disabled = p.inset;
    c.shadow = p.shadow;
    c.error = p.error;
    theme.set("writer_palette", p);
    theme.set(
        "input",
        InputColorsThemePreference {
            background: Preference::Specific(p.inset),
            focus_background: Preference::Specific(p.surface),
            color: Preference::Specific(p.ink),
            placeholder_color: Preference::Specific(p.placeholder),
            border_fill: Preference::Specific(p.boundary),
            focus_border_fill: Preference::Specific(p.accent),
        },
    );
    theme.set(
        "input_layout",
        InputLayoutThemePreference {
            corner_radius: Preference::Specific(t::RADIUS.into()),
            padding: Preference::Specific(Gaps::new(7., 10., 7., 10.)),
        },
    );
    theme.set(
        "menu_container",
        MenuContainerThemePreference {
            background: Preference::Specific(p.floating),
            padding: Preference::Specific(t::SPACE_XS.into()),
            shadow: Preference::Specific(p.shadow),
            border_fill: Preference::Specific(p.rule),
            corner_radius: Preference::Specific(t::DIALOG_RADIUS.into()),
        },
    );
    theme.set(
        "scrollbar",
        ScrollBarThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            thumb_background: Preference::Specific(p.boundary),
            hover_thumb_background: Preference::Specific(p.muted),
            active_thumb_background: Preference::Specific(p.accent),
            size: Preference::Specific(8.),
        },
    );
    theme.set(
        "tooltip",
        TooltipThemePreference {
            background: Preference::Specific(p.floating),
            color: Preference::Specific(p.ink),
            border_fill: Preference::Specific(p.rule),
            font_size: Preference::Specific(t::small()),
        },
    );
    let mut editor = if dark {
        EditorTheme::dark()
    } else {
        EditorTheme::light()
    };
    editor.background = p.surface;
    editor.text = p.ink;
    editor.cursor = p.ink;
    editor.line_selected_background = p.hover;
    editor.highlight = p.selection;
    theme.set("code_editor", EditorThemePreference::from(editor));
    theme.set("code_editor_syntax", syntax(dark));
    theme
}
