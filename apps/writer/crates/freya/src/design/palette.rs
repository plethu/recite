use freya::{code_editor::*, prelude::*};

pub fn theme(dark: bool) -> Theme {
    let mut theme = if dark {
        dark_theme().with_dark_code_editor()
    } else {
        light_theme().with_light_code_editor()
    };
    let colors = &mut theme.colors;
    colors.primary = color(dark, (72, 99, 77), (180, 203, 164));
    colors.background = color(dark, (245, 242, 237), (32, 33, 36));
    colors.surface_primary = color(dark, (236, 232, 226), (48, 50, 55));
    colors.surface_secondary = colors.surface_primary;
    colors.surface_tertiary = color(dark, (255, 253, 250), (41, 42, 45));
    colors.text_primary = text(dark);
    colors.text_secondary = color(dark, (101, 97, 91), (188, 184, 177));
    colors.border = color(dark, (130, 125, 117), (162, 159, 153));
    colors.border_focus = colors.primary;
    colors.focus = color(dark, (224, 232, 220), (62, 77, 64));
    theme.set("code_editor", EditorThemePreference::from(editor(dark)));
    theme.set("code_editor_syntax", syntax(dark));
    theme
}

pub fn syntax(dark: bool) -> EditorSyntaxTheme {
    let mut theme = if dark {
        EditorSyntaxTheme::dark()
    } else {
        EditorSyntaxTheme::light()
    };
    theme.text = text(dark);
    theme.keyword = color(dark, (54, 95, 146), (166, 200, 240));
    theme.punctuation_special = theme.keyword;
    theme.label = color(dark, (114, 82, 139), (210, 181, 236));
    theme.variable = theme.label;
    theme.function = color(dark, (38, 105, 103), (138, 207, 198));
    theme.function_method = theme.function;
    theme.constant = color(dark, (134, 85, 32), (230, 190, 133));
    theme.string = theme.constant;
    theme.string_special = theme.text;
    theme.number = theme.constant;
    theme.comment = color(dark, (101, 97, 91), (188, 184, 177));
    theme.property = theme.comment;
    theme
}

fn editor(dark: bool) -> EditorTheme {
    let mut editor = if dark {
        EditorTheme::dark()
    } else {
        EditorTheme::light()
    };
    editor.background = color(dark, (255, 253, 250), (41, 42, 45));
    editor.text = text(dark);
    editor.cursor = editor.text;
    editor.line_selected_background = color(dark, (236, 232, 226), (48, 50, 55));
    editor.highlight = color(dark, (224, 232, 220), (62, 77, 64));
    editor
}

fn color(dark: bool, light: (u8, u8, u8), night: (u8, u8, u8)) -> Color {
    let (r, g, b) = if dark { night } else { light };
    Color::from_rgb(r, g, b)
}

pub fn reading(dark: bool) -> Color {
    color(dark, (255, 253, 250), (41, 42, 45))
}
pub fn muted(dark: bool) -> Color {
    color(dark, (101, 97, 91), (188, 184, 177))
}
pub fn rule(dark: bool) -> Color {
    color(dark, (215, 210, 202), (73, 74, 77))
}
pub fn accent(dark: bool) -> Color {
    color(dark, (72, 99, 77), (180, 203, 164))
}
pub fn selection(dark: bool) -> Color {
    color(dark, (224, 232, 220), (62, 77, 64))
}

pub fn display_name(identifier: &str) -> String {
    let name = identifier
        .strip_suffix(".recite")
        .unwrap_or(identifier)
        .replace('_', " ");
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn text(dark: bool) -> Color {
    color(dark, (50, 47, 43), (234, 231, 225))
}
