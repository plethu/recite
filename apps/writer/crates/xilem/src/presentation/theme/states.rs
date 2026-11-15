//! One table of control states: rest, hover, pressed, focus and disabled for every class.
//!
//! Every state keeps the control's border box: a wider border takes its width from the padding,
//! and a pressed face sinks one pixel down and right inside the same box.
use super::super::control_style::ControlStyle;
use super::super::tokens::{Type, size};
use super::*;
use recite_writer_session::palette::{
    INK_HOVER, PRIMARY_HOVER, PRIMARY_PRESSED, STAMP_HOVER, STAMP_PRESSED, TONAL, TONAL_DISABLED,
    TONAL_HOVER,
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
}

/// The focus and pressed border width.
const STRONG: f64 = 2.;

impl Look {
    fn table(p: &Palette<Color>, scale: f64) -> Vec<(Option<ControlStyle>, Self)> {
        let ink_hover = p.ink.with_alpha(INK_HOVER);
        // A text control is a row tall: one Control line between its padding and a 1px border.
        let line = f64::from(Type::Control.metrics().1);
        let text = (((size::ROW - line) * scale - 2.) / 2., 12. * scale);
        let quiet = |padding| Self {
            rest: Some((Color::TRANSPARENT, Color::TRANSPARENT)),
            hover: (ink_hover, Color::TRANSPARENT),
            pressed: (p.pressed, p.ink),
            focus: Some(p.accent),
            disabled: (Color::TRANSPARENT, Color::TRANSPARENT),
            border: 1.,
            sinks: true,
            padding,
        };
        let stamp = |amount| p.stamp.lerp_rect(p.on_stamp, amount);
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
        };
        vec![
            (None, tonal()),
            (Some(ControlStyle::Tonal), tonal()),
            (Some(ControlStyle::Action), quiet(text)),
            (Some(ControlStyle::Navigation), quiet(text)),
            (Some(ControlStyle::Segment), quiet(text)),
            (Some(ControlStyle::Manuscript), quiet(text)),
            (Some(ControlStyle::Icon), quiet((4. * scale, 4. * scale))),
            (
                Some(ControlStyle::Selected),
                Self {
                    rest: Some((p.stamp, p.stamp)),
                    hover: (stamp(STAMP_HOVER), stamp(STAMP_HOVER)),
                    pressed: (stamp(STAMP_PRESSED), stamp(STAMP_PRESSED)),
                    focus: None,
                    // The current choice stays legible while the group waits.
                    disabled: (p.stamp, p.stamp),
                    border: 1.,
                    sinks: true,
                    padding: text,
                },
            ),
            (
                Some(ControlStyle::Primary),
                Self {
                    rest: Some((p.accent, p.accent)),
                    hover: (p.accent.with_alpha(PRIMARY_HOVER), p.accent),
                    pressed: (p.accent.with_alpha(PRIMARY_PRESSED), p.accent),
                    focus: None,
                    disabled: (p.inset, p.rule),
                    border: STRONG,
                    sinks: true,
                    padding: text,
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
            left: (horizontal - take + sink).px(),
            right: (horizontal - take - sink).px(),
        }
    }

    fn push(&self, stack: &mut PropertyStack, selector: &Selector) {
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
                .with(BorderWidth { width: STRONG.px() })
                .with(self.padding(STRONG, 0.)),
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
                .with(BorderWidth { width: STRONG.px() })
                .with(self.padding(STRONG, 1.));
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

/// Buttons and segment radios: the whole table, unclassed first so classes shadow it.
pub(super) fn pressable<W: Widget>(
    properties: &mut DefaultProperties,
    p: &Palette<Color>,
    scale: f64,
) {
    let mut stack = PropertyStack::new();
    for (style, look) in Look::table(p, scale) {
        let selector =
            style.map_or_else(Selector::new, |style| Selector::classes(&[style.class()]));
        look.push(&mut stack, &selector);
    }
    properties.insert_stack::<W>(stack);
}

/// Text inputs and checkboxes: opaque hover so a field never shows the desk, and no sinking.
/// Manuscript fields have no frame at rest and a hairline of padding for the 2px focus border.
pub(super) fn field<W: Widget>(properties: &mut DefaultProperties, p: &Palette<Color>, scale: f64) {
    let look = |rest, border: Color, padding| Look {
        rest,
        hover: (p.hover, border),
        pressed: (p.pressed, border),
        focus: Some(p.accent),
        disabled: (
            rest.map_or(p.inset, |(fill, _)| fill),
            rest.map_or(p.rule, |(_, border)| border),
        ),
        border: 1.,
        sinks: false,
        padding,
    };
    let mut stack = PropertyStack::new();
    look(None, p.boundary, (6. * scale, 12. * scale)).push(&mut stack, &Selector::new());
    look(
        Some((Color::TRANSPARENT, Color::TRANSPARENT)),
        Color::TRANSPARENT,
        (1., 1.),
    )
    .push(
        &mut stack,
        &Selector::classes(&[ControlStyle::Manuscript.class()]),
    );
    properties.insert_stack::<W>(stack);
}
