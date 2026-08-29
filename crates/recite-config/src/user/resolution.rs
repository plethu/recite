//! Deterministic field ownership and precedence for user-facing settings.

use super::model::{
    ConfigAuthority, KeyHints, Keymap, PlayConfig, TuiColorMode, TuiContrast, UserConfig,
    UserConfigField,
};
use recite_ui::UiLocale;
use thiserror::Error;

/// The source selected for a resolved field, including in-memory defaults.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FieldProvenance {
    /// No authority supplied a value; the named policy default was selected.
    Default,
    /// An explicit authority supplied a value.
    Authority(ConfigAuthority),
}

/// A value together with the authority that supplied it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityValue<T> {
    authority: ConfigAuthority,
    value: T,
}

impl<T> AuthorityValue<T> {
    /// Creates one candidate without applying precedence.
    #[must_use]
    pub const fn new(authority: ConfigAuthority, value: T) -> Self {
        Self { authority, value }
    }

    /// Returns the candidate's authority.
    #[must_use]
    pub const fn authority(&self) -> ConfigAuthority {
        self.authority
    }

    /// Returns the candidate value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }
}

/// A selected field value and its deterministic provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedField<T> {
    value: T,
    provenance: FieldProvenance,
}

impl<T> ResolvedField<T> {
    /// Returns the selected value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the selected value's provenance.
    #[must_use]
    pub const fn provenance(&self) -> FieldProvenance {
        self.provenance
    }
}

/// Failure to apply a named field policy to authority candidates.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
#[non_exhaustive]
pub enum FieldResolutionError {
    /// The authority is not allowed to provide this field.
    #[error("{authority:?} cannot provide {field:?}")]
    ForbiddenAuthority {
        /// Rejected authority.
        authority: ConfigAuthority,
        /// Field whose policy rejected it.
        field: UserConfigField,
    },
    /// More than one candidate from the same authority was supplied.
    #[error("{authority:?} supplied {field:?} more than once")]
    DuplicateAuthority {
        /// Duplicated authority.
        authority: ConfigAuthority,
        /// Field with the duplicate candidates.
        field: UserConfigField,
    },
}

mod sealed {
    pub trait Sealed {}
}

/// A sealed, named policy for one settled user-owned field.
pub trait FieldPolicy: sealed::Sealed + Copy {
    /// The field's value type.
    type Value: Clone;

    /// Returns the field covered by this policy.
    fn field(self) -> UserConfigField;

    /// Returns whether this authority may supply this field.
    fn allows(self, authority: ConfigAuthority) -> bool;
}

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

/// Resolve candidates with a named policy. Invocation wins only for the one
/// policy that explicitly permits invocation; user and generated values never
/// become project-semantic fallbacks because no project policy is exposed here.
pub fn resolve_field<P>(
    policy: P,
    default: P::Value,
    candidates: impl IntoIterator<Item = AuthorityValue<P::Value>>,
) -> Result<ResolvedField<P::Value>, FieldResolutionError>
where
    P: FieldPolicy,
{
    let mut invocation = None;
    let mut user = None;
    for candidate in candidates {
        if !policy.allows(candidate.authority) {
            return Err(FieldResolutionError::ForbiddenAuthority {
                authority: candidate.authority,
                field: policy.field(),
            });
        }
        match candidate.authority {
            ConfigAuthority::Invocation => {
                if invocation.replace(candidate.value).is_some() {
                    return Err(FieldResolutionError::DuplicateAuthority {
                        authority: ConfigAuthority::Invocation,
                        field: policy.field(),
                    });
                }
            }
            ConfigAuthority::User => {
                if user.replace(candidate.value).is_some() {
                    return Err(FieldResolutionError::DuplicateAuthority {
                        authority: ConfigAuthority::User,
                        field: policy.field(),
                    });
                }
            }
            ConfigAuthority::Project | ConfigAuthority::Generated => unreachable!(
                "the policy must reject project and generated authorities before selection"
            ),
        }
    }

    if let Some(value) = invocation {
        return Ok(ResolvedField {
            value,
            provenance: FieldProvenance::Authority(ConfigAuthority::Invocation),
        });
    }
    if let Some(value) = user {
        return Ok(ResolvedField {
            value,
            provenance: FieldProvenance::Authority(ConfigAuthority::User),
        });
    }
    Ok(ResolvedField {
        value: default,
        provenance: FieldProvenance::Default,
    })
}

/// Invocation-owned overrides. The type exposes only the already-settled
/// invocation override, keymap; presentation fields remain user-only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InvocationOverrides {
    keymap: Option<Keymap>,
}

impl InvocationOverrides {
    /// Creates an invocation with no overrides.
    #[must_use]
    pub const fn new() -> Self {
        Self { keymap: None }
    }

    /// Sets the invocation-owned keymap override.
    #[must_use]
    pub const fn with_keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = Some(keymap);
        self
    }

    /// Returns the optional invocation keymap override.
    #[must_use]
    pub const fn keymap(&self) -> Option<Keymap> {
        self.keymap
    }
}

/// Fully resolved user presentation settings with per-field provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedUserConfig {
    ui_locale: ResolvedField<UiLocale>,
    keymap: ResolvedField<Keymap>,
    key_hints: ResolvedField<KeyHints>,
    color: ResolvedField<TuiColorMode>,
    contrast: ResolvedField<TuiContrast>,
    show_unavailable_choices: ResolvedField<bool>,
}

impl ResolvedUserConfig {
    /// Returns the resolved UI settings.
    #[must_use]
    pub const fn ui(&self) -> ResolvedUiConfig<'_> {
        ResolvedUiConfig { config: self }
    }

    /// Returns the resolved play setting.
    #[must_use]
    pub const fn show_unavailable_choices(&self) -> &ResolvedField<bool> {
        &self.show_unavailable_choices
    }
}

/// Borrowed resolved UI settings.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedUiConfig<'a> {
    config: &'a ResolvedUserConfig,
}

impl ResolvedUiConfig<'_> {
    /// Returns the resolved locale.
    #[must_use]
    pub const fn locale(&self) -> &ResolvedField<UiLocale> {
        &self.config.ui_locale
    }

    /// Returns the resolved keymap.
    #[must_use]
    pub const fn keymap(&self) -> &ResolvedField<Keymap> {
        &self.config.keymap
    }

    /// Returns the resolved key-hint preference.
    #[must_use]
    pub const fn key_hints(&self) -> &ResolvedField<KeyHints> {
        &self.config.key_hints
    }

    /// Returns the resolved colour preference.
    #[must_use]
    pub const fn color(&self) -> &ResolvedField<TuiColorMode> {
        &self.config.color
    }

    /// Returns the resolved contrast preference.
    #[must_use]
    pub const fn contrast(&self) -> &ResolvedField<TuiContrast> {
        &self.config.contrast
    }
}

/// Apply the named user-field policies. No project or generated values enter
/// this resolution graph, keeping dialogue semantics outside user config.
pub fn resolve_user_config(
    config: &UserConfig,
    invocation: &InvocationOverrides,
) -> ResolvedUserConfig {
    let ui = &config.ui;
    let play = &config.play;
    ResolvedUserConfig {
        ui_locale: resolve_field(
            UiLocalePolicy,
            UiLocale::default(),
            [AuthorityValue::new(
                ConfigAuthority::User,
                ui.locale.clone(),
            )],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        keymap: resolve_field(
            KeymapPolicy,
            Keymap::default(),
            std::iter::once(AuthorityValue::new(ConfigAuthority::User, ui.keymap)).chain(
                invocation
                    .keymap
                    .into_iter()
                    .map(|value| AuthorityValue::new(ConfigAuthority::Invocation, value)),
            ),
        )
        .unwrap_or_else(|_| {
            unreachable!("the named keymap policy permits invocation and user values")
        }),
        key_hints: resolve_field(
            KeyHintsPolicy,
            KeyHints::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.key_hints)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        color: resolve_field(
            ColorPolicy,
            TuiColorMode::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.color)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        contrast: resolve_field(
            ContrastPolicy,
            TuiContrast::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.contrast)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        show_unavailable_choices: resolve_field(
            ShowUnavailableChoicesPolicy,
            PlayConfig::default().show_unavailable_choices,
            [AuthorityValue::new(
                ConfigAuthority::User,
                play.show_unavailable_choices,
            )],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
    }
}
