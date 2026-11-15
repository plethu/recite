//! Wells and indicators: text and number fields, and the checkbox and plain radio indicator.
use super::*;
use crate::presentation::tokens::space;

/// Text inputs: a tonal well closed by a bottom rule, a 2px accent ring around the whole well on
/// focus, and no sinking. Wells on the grained desk take the desk's rule, and search fields leave
/// room for their leading icon. Manuscript fields have no frame at rest and a hairline of padding
/// for the 2px focus border.
pub(in super::super) fn field<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let well = |rule, lead| Look {
        lead,
        ..Look::well(p, (6. * scale, size::CONTROL_PAD * scale), rule)
    };
    let lead = super::super::search::LEAD * scale;
    let (desk, search) = (
        ControlStyle::DeskField.class(),
        ControlStyle::SearchField.class(),
    );
    let mut stack = PropertyStack::new();
    for (selector, rule, lead) in [
        (Selector::new(), p.well_rule, 0.),
        (Selector::classes(&[desk]), p.desk_well_rule, 0.),
        (Selector::classes(&[search]), p.well_rule, lead),
        (Selector::classes(&[desk, search]), p.desk_well_rule, lead),
    ] {
        well(rule, lead).push(&mut stack, &selector, STRONG);
    }
    Look {
        rest: Some((Color::TRANSPARENT, Color::TRANSPARENT)),
        hover: (p.hover, Color::TRANSPARENT),
        pressed: (p.pressed, Color::TRANSPARENT),
        focus: Some(p.accent),
        disabled: (Color::TRANSPARENT, Color::TRANSPARENT),
        border: 1.,
        sinks: false,
        padding: (1., 1.),
        rule: Some(Color::TRANSPARENT),
        lead: 0.,
    }
    .push(
        &mut stack,
        &Selector::classes(&[ControlStyle::Manuscript.class()]),
        STRONG,
    );
    properties.insert_stack::<W>(stack);
}

/// Number fields: the paper well, padded so a stepper target and the well's edges make one row.
pub(in super::super) fn number<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let target = 24.;
    let mut stack = PropertyStack::new();
    Look::well(
        p,
        (
            ((size::ROW - target) * scale - 2.) / 2.,
            space::UNIT * scale,
        ),
        p.well_rule,
    )
    .push(&mut stack, &Selector::new(), STRONG);
    properties.insert_stack::<W>(stack);
}

/// Checkboxes and plain radios: no chip at rest, a full-row ink hover, and the indicator drawn in
/// the boundary colour. The toolkit strokes its focus and hover ring in the border colour, so
/// focus turns both accent.
pub(super) fn indicator(p: &Palette<Color>, scale: f64) -> Look {
    Look {
        rest: Some((Color::TRANSPARENT, p.boundary)),
        hover: (p.ink.with_alpha(INK_HOVER), p.boundary),
        pressed: (p.pressed, p.boundary),
        focus: Some(p.accent),
        disabled: (Color::TRANSPARENT, p.rule),
        border: 1.,
        sinks: false,
        padding: (6. * scale, size::CONTROL_PAD * scale),
        rule: None,
        lead: 0.,
    }
}

pub(in super::super) fn check<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let mut stack = PropertyStack::new();
    indicator(p, scale).push(&mut stack, &Selector::new(), STRONG);
    Look {
        // The least that leaves room for the focus and pressed border.
        padding: (6. * scale, 2.),
        ..indicator(p, scale)
    }
    .push(
        &mut stack,
        &Selector::classes(&[crate::presentation::control_style::ControlStyle::Flush.class()]),
        STRONG,
    );
    properties.insert_stack::<W>(stack);
}
