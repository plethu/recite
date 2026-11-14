use super::*;

fn luminance(color: Rgba) -> f64 {
    let linear = |channel: u8| {
        let channel = f64::from(channel) / 255.;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
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
        (p.teal_text.r, p.teal_text.g, p.teal_text.b),
        (15, 107, 111)
    );
    assert_eq!((p.accent.r, p.accent.g, p.accent.b), (104, 62, 98));
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

#[test]
fn monochrome_retains_alpha_and_srgb_interpolation_clamps_and_rounds_channels() {
    let a = Rgba::from_argb(110, 104, 62, 98);
    let gray = a.monochrome();
    assert_eq!(gray.a, 110);
    assert_eq!(gray.r, gray.g);
    assert_eq!(gray.g, gray.b);
    assert_eq!(Rgba::mix(a, Rgba::TRANSPARENT, -1.), a);
    assert_eq!(Rgba::mix(a, Rgba::TRANSPARENT, 2.), Rgba::TRANSPARENT);
    assert_eq!(
        Rgba::mix(
            Rgba::from_argb(0, 0, 0, 0),
            Rgba::from_argb(255, 255, 255, 255),
            0.5
        ),
        Rgba::from_argb(128, 128, 128, 128)
    );
}

#[test]
fn desk_text_survives_the_grain_and_the_sheet_shadow_stays_decorative() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let desk = p.grained_desk();
        // Chrome text on the desk: sidebar captions, tab row, toolbar and welcome copy.
        for text in [p.ink, p.muted, p.accent, p.teal_text] {
            assert!(
                contrast(text, desk) >= 4.5,
                "desk text: dark={dark} monochrome={monochrome} {text:?}: {}",
                contrast(text, desk)
            );
        }
        assert!(
            contrast(p.boundary, desk) >= 3.,
            "control boundary on the desk: dark={dark} monochrome={monochrome}"
        );
        // The sheet's offset shadow is visible against the desk but never carries meaning.
        let shadow = contrast(p.stock, p.canvas);
        assert!(
            (1.3..3.).contains(&shadow),
            "stock: dark={dark} monochrome={monochrome}: {shadow}"
        );
        assert!(contrast(p.stock, p.canvas) < contrast(p.boundary, p.canvas));
    }
}

#[test]
fn hover_ink_beats_the_grain_and_filled_focus_rings_contrast_with_their_fill() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let desk = p.grained_desk();
        for base in [desk, p.canvas, p.surface] {
            let hover = p.hover_over(base);
            assert!(
                contrast(p.ink, hover) >= 9.,
                "hover ink: dark={dark} monochrome={monochrome} {base:?}: {}",
                contrast(p.ink, hover)
            );
            for text in [p.muted, p.accent] {
                assert!(
                    contrast(text, hover) >= 4.5,
                    "hover text: {text:?} on {base:?}"
                );
            }
            // The hover must read as more than the grain's own variation.
            assert!(contrast(hover, base) > contrast(desk, p.canvas));
        }
        assert!(contrast(p.teal_text, p.hover_over(p.surface)) >= 4.5);
        // A filled face's ring is its own text colour, never a border close to the fill.
        assert!(contrast(p.on_accent, p.accent) >= 4.5);
        for amount in [0., STAMP_HOVER, STAMP_PRESSED] {
            assert!(contrast(p.on_stamp, p.stamp_state(amount)) >= 4.5);
        }
        assert!(
            contrast(p.accent, p.stamp) < 3.,
            "why the accent is not the ring on a stamp"
        );
    }
}
