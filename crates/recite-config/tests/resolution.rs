#![allow(clippy::expect_used)]

use recite_config::{
    AuthorityValue, ConfigAuthority, FieldProvenance, FieldResolutionError, InvocationOverrides,
    Keymap, KeymapPolicy, UiLocale, UiLocalePolicy, UserConfig, resolve_field, resolve_user_config,
};

#[test]
fn defaults_have_explicit_default_provenance() {
    let resolved = resolve_user_config(&UserConfig::default(), &InvocationOverrides::new());

    assert_eq!(
        resolved.ui().keymap().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    assert_eq!(
        resolved.ui().locale().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    assert_eq!(
        resolve_field(KeymapPolicy, Keymap::Standard, [])
            .expect("no candidates")
            .provenance(),
        FieldProvenance::Default
    );
}

#[test]
fn invocation_precedes_user_only_for_the_keymap_policy() {
    let config = UserConfig {
        ui: recite_config::UiConfig {
            keymap: Keymap::Standard,
            ..recite_config::UiConfig::default()
        },
        ..UserConfig::default()
    };
    let resolved = resolve_user_config(
        &config,
        &InvocationOverrides::new().with_keymap(Keymap::Vim),
    );

    assert_eq!(resolved.ui().keymap().value(), &Keymap::Vim);
    assert_eq!(
        resolved.ui().keymap().provenance(),
        FieldProvenance::Authority(ConfigAuthority::Invocation)
    );
}

#[test]
fn non_overridable_and_non_user_authorities_are_rejected() {
    let invocation_locale = resolve_field(
        UiLocalePolicy,
        UiLocale::default(),
        [AuthorityValue::new(
            ConfigAuthority::Invocation,
            UiLocale::parse("fr-FR").expect("locale"),
        )],
    );
    assert_eq!(
        invocation_locale,
        Err(FieldResolutionError::ForbiddenAuthority {
            authority: ConfigAuthority::Invocation,
            field: recite_config::UserConfigField::UiLocale,
        })
    );

    for authority in [ConfigAuthority::Project, ConfigAuthority::Generated] {
        let rejected = resolve_field(
            KeymapPolicy,
            Keymap::Standard,
            [AuthorityValue::new(authority, Keymap::Vim)],
        );
        assert!(matches!(
            rejected,
            Err(FieldResolutionError::ForbiddenAuthority { authority: found, .. })
                if found == authority
        ));
    }
}

#[test]
fn duplicate_authority_is_rejected_instead_of_last_write_wins() {
    let result = resolve_field(
        KeymapPolicy,
        Keymap::Standard,
        [
            AuthorityValue::new(ConfigAuthority::User, Keymap::Standard),
            AuthorityValue::new(ConfigAuthority::User, Keymap::Vim),
        ],
    );
    assert_eq!(
        result,
        Err(FieldResolutionError::DuplicateAuthority {
            authority: ConfigAuthority::User,
            field: recite_config::UserConfigField::Keymap,
        })
    );
}
