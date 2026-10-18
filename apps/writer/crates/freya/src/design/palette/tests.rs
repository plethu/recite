use super::*;
use crate::design::tokens::{STAMP_HOVER, STAMP_PRESSED};

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

#[test]
fn second_ink_and_overprint_keep_text_on_safe_surfaces() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for text in [p.ink, p.muted, p.accent] {
            assert!(
                contrast(text, p.plate) >= 4.5,
                "plate text: dark={dark} monochrome={monochrome}"
            );
        }
        // Destinations may sit on reading surfaces, hover and the plate, but not a pressed fill.
        for background in [
            p.canvas,
            p.surface,
            p.inset,
            p.floating,
            p.hover,
            p.selection,
            p.plate,
        ] {
            assert!(
                contrast(p.teal_text, background) >= 4.5,
                "teal text: dark={dark} monochrome={monochrome}, {background:?}: {}",
                contrast(p.teal_text, background)
            );
        }
        for background in [p.canvas, p.surface] {
            assert!(
                contrast(p.teal, background) >= 3.,
                "teal rule: dark={dark} monochrome={monochrome}"
            );
        }
        for text in [p.status_text, p.status_signal] {
            assert!(
                contrast(text, p.status_bar) >= 4.5,
                "status text: dark={dark} monochrome={monochrome}"
            );
        }
        for pressed in [false, true] {
            let background = if pressed { p.pressed } else { p.hover };
            assert!(
                contrast(p.destination(pressed), background) >= 4.5,
                "destination: dark={dark} monochrome={monochrome} pressed={pressed}"
            );
        }
        assert_eq!(p.destination(true), p.ink);
        for amount in [0., STAMP_HOVER, STAMP_PRESSED] {
            assert!(
                contrast(p.on_stamp, p.stamp_state(amount)) >= 4.5,
                "stamp text: dark={dark} monochrome={monochrome} amount={amount}"
            );
        }
        for background in [p.canvas, p.surface, p.floating] {
            assert!(
                contrast(p.stamp, background) >= 3.,
                "stamp stroke: dark={dark} monochrome={monochrome}"
            );
        }
        assert!(contrast(p.plate, p.surface) > contrast(p.hover, p.surface));
    }
}

#[test]
fn light_second_ink_text_is_the_text_safe_teal() {
    let p = Palette::new(false);
    assert!(contrast(p.teal, p.canvas) < 4.5, "riso teal is a plate ink");
    assert_eq!(
        (p.teal_text.r(), p.teal_text.g(), p.teal_text.b()),
        (15, 107, 111)
    );
    assert_eq!((p.accent.r(), p.accent.g(), p.accent.b()), (104, 62, 98));
}

#[test]
fn dark_surfaces_separate_without_borders() {
    let p = Palette::new(true);
    assert!(contrast(p.canvas, p.surface) >= 1.15);
    // Navy on dark paper is 1.05:1, so the dark status bar is a floating panel instead.
    assert!(contrast(p.status_bar, p.canvas) >= 1.15);
    assert!(contrast(p.surface, p.floating) >= 1.15);
    for (a, b) in [
        (p.hover, p.selection),
        (p.selection, p.pressed),
        (p.hover, p.floating),
    ] {
        assert!(
            contrast(a, b) > 1.04,
            "dark interaction plates: {a:?} {b:?}"
        );
    }
}
