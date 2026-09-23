use super::*;

fn luminance(color: Color) -> f64 {
    let linear = |channel: u8| {
        let channel = f64::from(channel) / 255.;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn body_and_supporting_text_remain_readable_across_surfaces_and_interactions() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for background in [
            p.canvas,
            p.surface,
            p.inset,
            p.floating,
            p.selection,
            p.hover,
            p.pressed,
        ] {
            for text in [p.ink, p.muted, p.error] {
                assert!(
                    contrast(text, background) >= 4.5,
                    "dark={dark}, text={text:?}, background={background:?}"
                );
                for edge in crate::design::material::face_edges(background, 0.) {
                    assert!(
                        contrast(text, edge) >= 4.5,
                        "gradient text contrast: dark={dark}"
                    );
                }
            }
        }
        for amount in [0., 0.08, 0.16] {
            assert!(contrast(p.on_accent, Color::lerp(p.accent, p.on_accent, amount)) >= 4.5);
            for edge in
                crate::design::material::face_edges(Color::lerp(p.accent, p.on_accent, amount), 0.)
            {
                assert!(
                    contrast(p.on_accent, edge) >= 4.5,
                    "primary gradient contrast: dark={dark}"
                );
            }
        }
        for surface in [p.surface, p.inset, p.floating] {
            assert!(
                contrast(p.placeholder, surface) >= 4.5,
                "placeholder contrast: dark={dark}"
            );
            assert!(contrast(p.accent, surface) >= 3.);
            assert!(contrast(p.boundary, surface) >= 3.);
        }
    }
}

#[test]
fn syntax_and_focus_contrast_cover_both_colour_modes() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let syntax = syntax_with_mode(dark, monochrome);
        for surface in [p.surface, p.inset, p.hover, p.selection] {
            let EditorSyntaxTheme {
                text,
                whitespace,
                attribute,
                boolean,
                comment,
                constant,
                constructor,
                escape,
                function,
                function_macro,
                function_method,
                keyword,
                label,
                module,
                number,
                operator,
                property,
                punctuation,
                punctuation_bracket,
                punctuation_delimiter,
                punctuation_special,
                string,
                string_escape,
                string_special,
                tag,
                text_literal,
                text_reference,
                text_title,
                text_uri,
                text_emphasis,
                type_,
                variable,
                variable_builtin,
                variable_parameter,
            } = syntax.clone();
            for ink in [
                text,
                whitespace,
                attribute,
                boolean,
                comment,
                constant,
                constructor,
                escape,
                function,
                function_macro,
                function_method,
                keyword,
                label,
                module,
                number,
                operator,
                property,
                punctuation,
                punctuation_bracket,
                punctuation_delimiter,
                punctuation_special,
                string,
                string_escape,
                string_special,
                tag,
                text_literal,
                text_reference,
                text_title,
                text_uri,
                text_emphasis,
                type_,
                variable,
                variable_builtin,
                variable_parameter,
            ] {
                assert!(
                    contrast(ink, surface) >= 4.5,
                    "syntax: dark={dark} monochrome={monochrome}, {ink:?} over {surface:?}: {}",
                    contrast(ink, surface)
                );
                if monochrome {
                    assert_eq!((ink.r(), ink.g()), (ink.g(), ink.b()));
                }
            }
        }
        for surface in [
            p.canvas,
            p.surface,
            p.inset,
            p.floating,
            p.selection,
            p.hover,
            p.pressed,
        ] {
            assert!(
                contrast(p.accent, surface) >= 3.,
                "focus: {dark} {monochrome}"
            );
        }
        assert!(
            contrast(p.boundary, p.canvas) >= 3.,
            "map edges: {dark} {monochrome}"
        );
    }
}
