use super::*;

fn contrast(a: Rgba, b: Rgba) -> f64 {
    Rgba::contrast(a, b)
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
        // The status words carry state, behind a modal too: no dim lies over the bar, and under
        // one its words fall to 2.2-5.6:1, so the bar keeps its own contrast.
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
    }
}

/// Every host a control may sit on: the desk at its darkest grain point and each paper.
fn hosts(p: &Palette) -> [Rgba; 5] {
    [p.grained_desk(), p.canvas, p.surface, p.inset, p.floating]
}

#[test]
fn tonal_actions_keep_their_text_through_rest_hover_and_press_on_every_host() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for host in hosts(&p) {
            for amount in [TONAL, TONAL_HOVER] {
                let fill = p.tint(host, amount);
                for text in [p.ink, p.muted, p.accent] {
                    assert!(
                        contrast(text, fill) >= 4.5,
                        "tonal text: dark={dark} monochrome={monochrome} {host:?} {amount}: {}",
                        contrast(text, fill)
                    );
                }
            }
            // Hover must read as a change of state on its own host.
            assert!(contrast(p.tint(host, TONAL_HOVER), p.tint(host, TONAL)) > 1.04);
            assert!(contrast(p.ink, p.pressed) >= 4.5);
        }
    }
}

#[test]
fn the_primary_keeps_its_text_in_both_modes_while_hovered_and_pressed() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for host in hosts(&p) {
            for alpha in [1., PRIMARY_HOVER, PRIMARY_PRESSED] {
                let fill = p.primary_over(host, alpha);
                assert!(
                    contrast(p.on_accent, fill) >= 4.5,
                    "primary text: dark={dark} monochrome={monochrome} {host:?} {alpha}: {}",
                    contrast(p.on_accent, fill)
                );
            }
            // The fill is its own boundary against the host.
            assert!(contrast(p.accent, host) >= 3., "primary fill: dark={dark}");
        }
    }
    // Dark mode's primary is the light accent with dark text; no control is a near-white stamp.
    let dark = Palette::new(true);
    assert_eq!(
        (dark.accent.r, dark.accent.g, dark.accent.b),
        (212, 174, 204)
    );
    assert_eq!(
        (dark.on_accent.r, dark.on_accent.g, dark.on_accent.b),
        (40, 25, 39)
    );
}

#[test]
fn a_field_well_is_identified_by_a_rule_of_three_to_one_on_every_host() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let desk = [p.grained_desk()];
        for (hosts, rule) in [
            (&p.papers()[..], p.field_rule(&p.papers())),
            (&desk[..], p.field_rule(&desk)),
        ] {
            for &host in hosts {
                let well = p.tint(host, WELL);
                for background in [host, well] {
                    assert!(
                        contrast(rule, background) >= 3.,
                        "field rule: dark={dark} monochrome={monochrome} {host:?}: {}",
                        contrast(rule, background)
                    );
                }
                // Text, and the muted hints that replace placeholders, hold on the well and its
                // hover.
                for fill in [well, p.hover_over(host)] {
                    for text in [p.ink, p.muted] {
                        assert!(contrast(text, fill) >= 4.5, "well text: {host:?}");
                    }
                }
            }
        }
        // Papers keep the quieter boundary; the light grained desk falls to ink (2.98:1).
        assert_eq!(p.field_rule(&p.papers()), p.boundary);
        if !dark {
            assert_eq!(p.field_rule(&desk), p.ink);
        }
        assert_eq!(
            (p.well_rule, p.desk_well_rule),
            (p.field_rule(&p.papers()), p.field_rule(&desk))
        );
    }
}

#[test]
fn mode_switch_marks_hold_three_to_one_where_they_sit() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let desk = p.grained_desk();
        // The chosen tab's underline sits on the desk at its darkest grain point.
        assert!(
            contrast(p.teal, desk) >= 3.,
            "tab underline: dark={dark} monochrome={monochrome}: {}",
            contrast(p.teal, desk)
        );
        // The chosen segment's ink bar sits on its floating-paper chip, over the tonal track on the desk.
        for background in [p.floating, p.tint(desk, TONAL), p.tint(p.canvas, TONAL)] {
            assert!(
                contrast(p.ink, background) >= 3.,
                "segment bar: dark={dark} monochrome={monochrome} {background:?}"
            );
        }
        // Chip text, chosen or not, on paper and on the track.
        for background in [p.floating, p.tint(desk, TONAL)] {
            for text in [p.ink, p.muted] {
                assert!(contrast(text, background) >= 4.5);
            }
        }
    }
}

#[test]
fn a_selected_row_keeps_its_text_and_underscore_on_the_plate_at_rest_and_hovered() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for fill in [p.plate, Rgba::mix(p.plate, p.ink, PLATE_HOVER)] {
            for text in [p.ink, p.muted, p.accent] {
                assert!(
                    contrast(text, fill) >= 4.5,
                    "row text: dark={dark} monochrome={monochrome} {fill:?}: {}",
                    contrast(text, fill)
                );
            }
            // The ink underscore under the label marks the selection beside the plate's tint.
            assert!(
                contrast(p.ink, fill) >= 3.,
                "row underscore: dark={dark} monochrome={monochrome} {fill:?}: {}",
                contrast(p.ink, fill)
            );
        }
    }
}

#[test]
fn a_modal_panel_is_edged_in_ink_that_holds_three_to_one_against_the_dim() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        // The dim lies over whatever the window shows: desk, papers and the darkest ink.
        for base in [p.grained_desk(), p.canvas, p.surface, p.floating] {
            for under in p.dimmed(base) {
                assert!(
                    contrast(p.ink, under) >= 3.,
                    "panel edge: dark={dark} monochrome={monochrome} {base:?}: {}",
                    contrast(p.ink, under)
                );
            }
        }
        // The screen is a light texture, never a second dim.
        assert!(p.screen.a <= 26 && p.backdrop.a >= 115);
    }
}

#[test]
fn dark_grain_is_a_faint_paper_tone_that_desk_text_survives() {
    let p = Palette::new(true);
    assert_eq!((p.grain.r, p.grain.g, p.grain.b), (243, 233, 223));
    assert!((0.04..=0.05).contains(&p.grain_peak));
    // Lighter than the canvas, so it reads as paper fibre rather than soot, but only just.
    let desk = p.grained_desk();
    assert!(desk.luminance() > p.canvas.luminance());
    assert!(contrast(desk, p.canvas) < 1.15);
    assert_eq!(Palette::new(false).grain_peak, GRAIN_PEAK);
}

#[test]
fn margin_marks_hold_three_to_one_wherever_they_repeat() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        let desk = p.grained_desk();
        // Crop marks sit in the sheet's pad and on the map's canvas around a card.
        for (host, name) in [(p.surface, "sheet"), (p.canvas, "canvas")] {
            assert!(
                contrast(p.crop, host) >= 3.,
                "crop marks on {name}: dark={dark} monochrome={monochrome}: {}",
                contrast(p.crop, host)
            );
        }
        // The unsaved dot sits in the sheet's pad, on a map card, in a scene row on the desk,
        // and beside a selected row's words on the plate.
        for (host, name) in [
            (p.surface, "sheet"),
            (p.canvas, "canvas"),
            (desk, "desk"),
            (p.plate, "selected row"),
        ] {
            assert!(
                contrast(p.unsaved, host) >= 3.,
                "unsaved dot on {name}: dark={dark} monochrome={monochrome}: {}",
                contrast(p.unsaved, host)
            );
        }
        // The problem wedge repeats on the sheet, in the scene list and on the selected row.
        for (host, name) in [
            (p.surface, "sheet"),
            (desk, "desk"),
            (p.plate, "selected row"),
        ] {
            assert!(
                contrast(p.error, host) >= 3.,
                "problem wedge on {name}: dark={dark} monochrome={monochrome}: {}",
                contrast(p.error, host)
            );
        }
        // Colour tells the marks apart, and without it the darkest ink keeps them legible.
        if monochrome {
            assert_eq!((p.crop, p.unsaved), (p.ink, p.ink));
        } else {
            assert_eq!((p.crop, p.unsaved), (p.teal, p.accent));
        }
    }
}

#[test]
fn a_status_bar_action_keeps_its_text_and_shows_focus_on_the_overprint() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for fill in [
            p.status_bar,
            Rgba::mix(p.status_bar, p.status_text, INK_HOVER),
            Rgba::mix(p.status_bar, p.status_text, TONAL_HOVER),
        ] {
            assert!(
                contrast(p.status_text, fill) >= 4.5,
                "status action text: dark={dark} monochrome={monochrome}: {}",
                contrast(p.status_text, fill)
            );
        }
        assert!(
            contrast(p.status_signal, p.status_bar) >= 3.,
            "status focus: dark={dark} monochrome={monochrome}: {}",
            contrast(p.status_signal, p.status_bar)
        );
    }
}

#[test]
fn a_scroll_bar_thumb_in_the_boundary_colour_holds_three_to_one_on_every_host() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        for host in hosts(&p) {
            assert!(
                contrast(p.boundary, host) >= 3.,
                "thumb on {host:?}: dark={dark} monochrome={monochrome}"
            );
        }
    }
}

#[test]
fn an_error_message_in_the_error_ink_reads_as_text_wherever_it_shows() {
    for (dark, monochrome) in [(false, false), (false, true), (true, false), (true, true)] {
        let p = Palette::with_mode(dark, monochrome);
        // A tool page and a slip are paper; a field's refused value sits beside its well.
        for (host, name) in [
            (p.surface, "page"),
            (p.floating, "slip"),
            (p.grained_desk(), "desk"),
        ] {
            assert!(
                contrast(p.error, host) >= 4.5,
                "error text on {name}: dark={dark} monochrome={monochrome}: {}",
                contrast(p.error, host)
            );
        }
    }
}
