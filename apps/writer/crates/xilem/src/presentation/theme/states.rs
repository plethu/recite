//! One table of control states: rest, hover, pressed, focus and disabled for every class.
//!
//! Every state keeps the control's border box: a wider border takes its width from the padding,
//! and a pressed face sinks one pixel down and right inside the same box.
use super::super::control_style::ControlStyle;
use super::super::tokens::{Type, size};
use super::*;
use recite_writer_session::palette::{
    INK_HOVER, PLATE_HOVER, PRIMARY_HOVER, PRIMARY_PRESSED, TONAL, TONAL_DISABLED, TONAL_HOVER,
    WELL,
};

/// How one kind of control looks in each state.
struct Look {
    rest: Option<(Color, Color)>,
    hover: (Color, Color),
    pressed: (Color, Color),
    /// A focus border; `None` for filled faces, which wear a [`super::material::focus_ring`].
    focus: Option<Color>,
    disabled: (Color, Color),
    border: f64,
    /// Whether pressing sinks the face; fields keep their text still under the pointer.
    sinks: bool,
    padding: (f64, f64),
    /// A field's 1px bottom rule, drawn as a hard shadow one pixel below the well; `None` leaves
    /// the shadow alone.
    rule: Option<Color>,
    /// Extra padding before the content, for a leading icon painted beneath it.
    lead: f64,
}

/// The focus and pressed border width.
const STRONG: f64 = 2.;

impl Look {
    fn table(p: &Palette<Color>, scale: f64) -> Vec<(Option<ControlStyle>, Self)> {
        let ink_hover = p.ink.with_alpha(INK_HOVER);
        // A text control is a row tall: one Control line between its padding and a 1px border.
        let line = f64::from(Type::Control.metrics().1);
        let text = (
            ((size::ROW - line) * scale - 2.) / 2.,
            size::CONTROL_PAD * scale,
        );
        let quiet = |padding| Self {
            rest: Some((Color::TRANSPARENT, Color::TRANSPARENT)),
            hover: (ink_hover, Color::TRANSPARENT),
            pressed: (p.pressed, p.ink),
            focus: Some(p.accent),
            disabled: (Color::TRANSPARENT, Color::TRANSPARENT),
            border: 1.,
            sinks: true,
            padding,
            rule: None,
            lead: 0.,
        };
        // Secondary actions on sheets, panels and dialogs: a tint of ink over the host, no border.
        let tonal = || Self {
            rest: Some((p.ink.with_alpha(TONAL), Color::TRANSPARENT)),
            hover: (p.ink.with_alpha(TONAL_HOVER), Color::TRANSPARENT),
            pressed: (p.pressed, p.ink),
            focus: Some(p.accent),
            disabled: (p.ink.with_alpha(TONAL_DISABLED), Color::TRANSPARENT),
            border: 1.,
            sinks: true,
            padding: text,
            rule: None,
            lead: 0.,
        };
        vec![
            (None, tonal()),
            (Some(ControlStyle::Tonal), tonal()),
            (Some(ControlStyle::Action), quiet(text)),
            (Some(ControlStyle::Navigation), quiet(text)),
            (Some(ControlStyle::Segment), quiet(text)),
            (Some(ControlStyle::Manuscript), quiet(text)),
            (Some(ControlStyle::Icon), quiet((4. * scale, 4. * scale))),
            (Some(ControlStyle::Tab), quiet(text)),
            (Some(ControlStyle::Row), quiet(text)),
            (
                Some(ControlStyle::Entry),
                Self {
                    rest: Some((p.ink.with_alpha(WELL), Color::TRANSPARENT)),
                    hover: (p.ink.with_alpha(INK_HOVER), Color::TRANSPARENT),
                    pressed: (p.pressed, p.ink),
                    focus: Some(p.accent),
                    disabled: (p.ink.with_alpha(TONAL_DISABLED), Color::TRANSPARENT),
                    border: 1.,
                    sinks: false,
                    padding: text,
                    rule: Some(p.desk_well_rule),
                    lead: 0.,
                },
            ),
            (
                Some(ControlStyle::RowSelected),
                Self {
                    rest: Some((p.plate, Color::TRANSPARENT)),
                    hover: (p.plate.lerp_rect(p.ink, PLATE_HOVER), Color::TRANSPARENT),
                    pressed: (p.pressed, p.ink),
                    focus: Some(p.accent),
                    disabled: (p.plate, Color::TRANSPARENT),
                    border: 1.,
                    sinks: true,
                    padding: text,
                    rule: None,
                    lead: 0.,
                },
            ),
            (
                Some(ControlStyle::Chosen),
                Self {
                    // Floating paper reads as a raised chip on every track: on the grained desk, on a
                    // flat page and on dark paper, where the reading surface matches the track.
                    rest: Some((p.floating, Color::TRANSPARENT)),
                    hover: (p.floating.lerp_rect(p.ink, INK_HOVER), Color::TRANSPARENT),
                    pressed: (p.pressed, p.ink),
                    focus: Some(p.accent),
                    // The current choice stays legible while the group waits.
                    disabled: (p.floating, Color::TRANSPARENT),
                    border: 1.,
                    sinks: true,
                    padding: text,
                    rule: None,
                    lead: 0.,
                },
            ),
            (
                Some(ControlStyle::Primary),
                Self {
                    rest: Some((p.accent, p.accent)),
                    hover: (p.accent.with_alpha(PRIMARY_HOVER), p.accent),
                    pressed: (p.accent.with_alpha(PRIMARY_PRESSED), p.accent),
                    focus: None,
                    // Disabled, it falls back to a faint tonal face: never an outline.
                    disabled: (p.ink.with_alpha(TONAL_DISABLED), Color::TRANSPARENT),
                    border: STRONG,
                    sinks: true,
                    padding: text,
                    rule: None,
                    lead: 0.,
                },
            ),
        ]
    }

    /// Padding for a border of `border` width, sunk by `sink` pixels down and right.
    fn padding(&self, border: f64, sink: f64) -> Padding {
        let (vertical, horizontal) = self.padding;
        let take = border - self.border;
        Padding {
            top: (vertical - take + sink).px(),
            bottom: (vertical - take - sink).px(),
            left: (horizontal + self.lead - take + sink).px(),
            right: (horizontal - take - sink).px(),
        }
    }

    /// Push this look's layers; `strong` is the focus and pressed border width.
    fn push(&self, stack: &mut PropertyStack, selector: &Selector, strong: f64) {
        let at = |hovered: Option<bool>, pressed: Option<bool>, focused: Option<bool>| {
            let mut selector = selector.clone();
            if let Some(value) = hovered {
                selector = selector.with_hovered(value);
            }
            if let Some(value) = pressed {
                selector = selector.with_active(value);
            }
            if let Some(value) = focused {
                selector = selector.with_focused(value);
            }
            selector
        };
        let mut rest = PropertySet::new()
            .with(BorderWidth {
                width: self.border.px(),
            })
            .with(CornerRadius { radius: 2.px() })
            .with(self.padding(self.border, 0.));
        if let Some((fill, border)) = self.rest {
            rest = rest
                .with(Background::Color(fill))
                .with(BorderColor { color: border });
        }
        if let Some(rule) = self.rule {
            rest = rest.with(BoxShadow::new(rule, (0., 1.)));
        }
        stack.push_layer(selector.clone(), rest);
        let (fill, border) = self.hover;
        stack.push_layer(
            at(Some(true), None, None),
            PropertySet::new()
                .with(Background::Color(fill))
                .with(BorderColor { color: border }),
        );
        // Every row shadows the unclassed focus border, so a filled face never inherits it.
        let focus = match (self.focus, self.rest) {
            (Some(color), _) => PropertySet::new()
                .with(BorderColor { color })
                .with(BorderWidth { width: strong.px() })
                .with(self.padding(strong, 0.)),
            (None, rest) => {
                let mut set = PropertySet::new()
                    .with(BorderWidth {
                        width: self.border.px(),
                    })
                    .with(self.padding(self.border, 0.));
                if let Some((_, color)) = rest {
                    set = set.with(BorderColor { color });
                }
                set
            }
        };
        stack.push_layer(at(None, None, Some(true)), focus);
        let (fill, border) = self.pressed;
        let mut pressed = PropertySet::new()
            .with(Background::Color(fill))
            .with(BorderColor { color: border });
        if self.sinks {
            pressed = pressed
                .with(BorderWidth { width: strong.px() })
                .with(self.padding(strong, 1.));
        }
        stack.push_layer(at(None, Some(true), None), pressed);
        let (fill, border) = self.disabled;
        stack.push_layer(
            selector.clone().with_disabled(true),
            PropertySet::new()
                .with(Background::Color(fill))
                .with(BorderColor { color: border })
                .with(BorderWidth {
                    width: self.border.px(),
                })
                .with(self.padding(self.border, 0.)),
        );
    }
}

/// Buttons: the whole table, unclassed first so classes shadow it.
pub(super) fn pressable<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let mut stack = PropertyStack::new();
    for (style, look) in Look::table(p, scale) {
        let selector =
            style.map_or_else(Selector::new, |style| Selector::classes(&[style.class()]));
        look.push(&mut stack, &selector, STRONG);
    }
    properties.insert_stack::<W>(stack);
}

/// Radios. A plain radio keeps its drawn indicator, whose outline is the border colour and width.
/// Segments and tabs hide the indicator and have no border: Masonry strokes a radio's focus ring
/// 2px outside it in the border colour and never paints the border itself, but it still strokes
/// the hidden indicator with the border's width, which at zero draws nothing.
pub(super) fn choosable<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let mut stack = PropertyStack::new();
    for (style, look) in Look::table(p, scale) {
        let Some(style) = style else {
            indicator(p, scale).push(&mut stack, &Selector::new(), STRONG);
            continue;
        };
        // The border's pixel moves into the padding, so the hit area stays the same.
        let (vertical, horizontal) = look.padding;
        Look {
            border: 0.,
            padding: (vertical + look.border, horizontal + look.border),
            ..look
        }
        .push(&mut stack, &Selector::classes(&[style.class()]), 0.);
    }
    properties.insert_stack::<W>(stack);
}

/// Text inputs: a tonal well closed by a bottom rule, a 2px accent ring around the whole well on
/// focus, and no sinking. Wells on the grained desk take the desk's rule, and search fields leave
/// room for their leading icon. Manuscript fields have no frame at rest and a hairline of padding
/// for the 2px focus border.
pub(super) fn field<W: Widget>(properties: &mut DefaultProperties, p: &Palette<Color>, scale: f64) {
    let well = |rule, lead| Look {
        rest: Some((p.ink.with_alpha(WELL), Color::TRANSPARENT)),
        hover: (p.ink.with_alpha(INK_HOVER), Color::TRANSPARENT),
        pressed: (p.ink.with_alpha(INK_HOVER), Color::TRANSPARENT),
        focus: Some(p.accent),
        disabled: (p.ink.with_alpha(TONAL_DISABLED), Color::TRANSPARENT),
        border: 1.,
        sinks: false,
        padding: (6. * scale, 12. * scale),
        rule: Some(rule),
        lead,
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
pub(super) fn number<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let target = 24.;
    let mut stack = PropertyStack::new();
    Look {
        rest: Some((p.ink.with_alpha(WELL), Color::TRANSPARENT)),
        hover: (p.ink.with_alpha(INK_HOVER), Color::TRANSPARENT),
        pressed: (p.ink.with_alpha(INK_HOVER), Color::TRANSPARENT),
        focus: Some(p.accent),
        disabled: (p.ink.with_alpha(TONAL_DISABLED), Color::TRANSPARENT),
        border: 1.,
        sinks: false,
        padding: (((size::ROW - target) * scale - 2.) / 2., 4. * scale),
        rule: Some(p.well_rule),
        lead: 0.,
    }
    .push(&mut stack, &Selector::new(), STRONG);
    properties.insert_stack::<W>(stack);
}

/// Checkboxes and plain radios: no chip at rest, a full-row ink hover, and the indicator drawn in
/// the boundary colour. The toolkit strokes its focus and hover ring in the border colour, so
/// focus turns both accent.
fn indicator(p: &Palette<Color>, scale: f64) -> Look {
    Look {
        rest: Some((Color::TRANSPARENT, p.boundary)),
        hover: (p.ink.with_alpha(INK_HOVER), p.boundary),
        pressed: (p.pressed, p.boundary),
        focus: Some(p.accent),
        disabled: (Color::TRANSPARENT, p.rule),
        border: 1.,
        sinks: false,
        padding: (6. * scale, 12. * scale),
        rule: None,
        lead: 0.,
    }
}

pub(super) fn check<W: Widget>(properties: &mut DefaultProperties, p: &Palette<Color>, scale: f64) {
    let mut stack = PropertyStack::new();
    indicator(p, scale).push(&mut stack, &Selector::new(), STRONG);
    properties.insert_stack::<W>(stack);
}
