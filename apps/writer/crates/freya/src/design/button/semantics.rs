//! A control's role and its applicable state are one value.
use accesskit::Toggled;
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Semantics {
    Button,
    Toggle { pressed: bool },
    CheckBox { checked: bool },
    Radio { selected: bool },
    Option { selected: bool },
    MenuItem,
    Tab { selected: bool },
}

impl Semantics {
    pub(super) fn is_radio(self) -> bool {
        matches!(self, Self::Radio { .. })
    }

    pub(super) fn apply(self, control: Rect) -> Rect {
        control
            .a11y_role(self.role())
            .a11y_builder(move |node| match self {
                Self::Button | Self::MenuItem => {}
                Self::Tab { selected } | Self::Option { selected } => node.set_selected(selected),
                Self::Toggle { pressed: checked }
                | Self::CheckBox { checked }
                | Self::Radio { selected: checked } => node.set_toggled(toggled(checked)),
            })
    }

    fn role(self) -> AccessibilityRole {
        match self {
            Self::Button | Self::Toggle { .. } => AccessibilityRole::Button,
            Self::CheckBox { .. } => AccessibilityRole::CheckBox,
            Self::Radio { .. } => AccessibilityRole::RadioButton,
            Self::Option { .. } => AccessibilityRole::ListBoxOption,
            Self::MenuItem => AccessibilityRole::MenuItem,
            Self::Tab { .. } => AccessibilityRole::Tab,
        }
    }
}

fn toggled(checked: bool) -> Toggled {
    if checked {
        Toggled::True
    } else {
        Toggled::False
    }
}
