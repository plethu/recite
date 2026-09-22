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
    for dark in [false, true] {
        let p = Palette::new(dark);
        for background in [
            p.canvas,
            p.surface,
            p.inset,
            p.floating,
            p.selection,
            p.hover,
            p.pressed,
        ] {
            for text in [p.ink, p.muted] {
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
