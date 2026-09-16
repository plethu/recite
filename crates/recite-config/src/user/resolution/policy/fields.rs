//! Field-specific authority declarations, separate from the resolution algorithm.
use super::{ConfigAuthority, FieldPolicy, UserConfigField, sealed};
use crate::{KeyHints, Keymap, TuiColorMode, TuiContrast, WriterPaneSide, WriterTheme, WriterView};
use recite_ui::UiLocale;

macro_rules! user_policy {
    ($name:ident, $field:expr, $value:ty, [$($allowed:pat),+]) => {
        #[doc = concat!("Sealed policy for the `", stringify!($field), "` user field.")]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;

        impl sealed::Sealed for $name {}

        impl FieldPolicy for $name {
            type Value = $value;

            fn field(self) -> UserConfigField {
                $field
            }

            fn allows(self, authority: ConfigAuthority) -> bool {
                matches!(authority, $($allowed)|+)
            }
        }
    };
}

user_policy!(
    UiLocalePolicy,
    UserConfigField::UiLocale,
    UiLocale,
    [ConfigAuthority::User]
);
user_policy!(
    KeymapPolicy,
    UserConfigField::Keymap,
    Keymap,
    [ConfigAuthority::Invocation, ConfigAuthority::User]
);
user_policy!(
    KeyHintsPolicy,
    UserConfigField::KeyHints,
    KeyHints,
    [ConfigAuthority::User]
);
user_policy!(
    ColorPolicy,
    UserConfigField::Color,
    TuiColorMode,
    [ConfigAuthority::User]
);
user_policy!(
    ContrastPolicy,
    UserConfigField::Contrast,
    TuiContrast,
    [ConfigAuthority::User]
);
user_policy!(
    ShowUnavailableChoicesPolicy,
    UserConfigField::ShowUnavailableChoices,
    bool,
    [ConfigAuthority::User]
);

user_policy!(
    WriterConfirmExitPolicy,
    UserConfigField::WriterConfirmExit,
    bool,
    [ConfigAuthority::User]
);

user_policy!(
    WriterViewPolicy,
    UserConfigField::WriterView,
    WriterView,
    [ConfigAuthority::User]
);
user_policy!(
    WriterThemePolicy,
    UserConfigField::WriterTheme,
    WriterTheme,
    [ConfigAuthority::User]
);
user_policy!(
    WriterReducedMotionPolicy,
    UserConfigField::WriterReducedMotion,
    bool,
    [ConfigAuthority::User]
);
user_policy!(
    WriterZoomToPointerPolicy,
    UserConfigField::WriterZoomToPointer,
    bool,
    [ConfigAuthority::User]
);

user_policy!(
    WriterPaneSidePolicy,
    UserConfigField::WriterPaneSide,
    WriterPaneSide,
    [ConfigAuthority::User]
);
