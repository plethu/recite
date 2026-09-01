#![expect(
    clippy::expect_used,
    reason = "resolution integration tests fail fast on temporary config and typed-value fixture setup; standalone test targets are outside clippy.toml's test allowance"
)]

use std::{fs, path::Path};

use recite_config::{
    AuthorityValue, ConfigAuthority, ConfigFormat, ConfigProvenance, FieldProvenance,
    FieldResolutionError, InvocationOverrides, Keymap, KeymapPolicy, Platform, PlatformRoots,
    UiLocale, UiLocalePolicy, UserConfig, load_user_config_from, resolve_field,
    resolve_user_config,
};
use tempfile::tempdir;

fn load_config(path: Option<&Path>) -> recite_config::LoadedUserConfig {
    load_user_config_from(Platform::Linux, &PlatformRoots::new(), path)
        .expect("test config fixture is valid")
}

#[test]
fn defaults_have_explicit_default_provenance() {
    let resolved = resolve_user_config(&load_config(None), &InvocationOverrides::new());

    assert_eq!(
        resolved.ui().keymap().provenance(),
        FieldProvenance::Default
    );
    assert_eq!(
        resolved.ui().locale().provenance(),
        FieldProvenance::Default
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
    let loaded = recite_config::LoadedUserConfig::from_explicit(UserConfig::default());
    let resolved = resolve_user_config(
        &loaded,
        &InvocationOverrides::new().with_keymap(Keymap::Vim),
    );

    assert_eq!(resolved.ui().keymap().value(), &Keymap::Vim);
    assert_eq!(
        resolved.ui().keymap().provenance(),
        FieldProvenance::Authority(ConfigAuthority::Invocation)
    );
}

#[test]
fn partial_and_default_equal_values_preserve_field_presence() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("config.toml");
    fs::write(
        &path,
        "config_version = 1\n[ui]\nlocale = \"en-US\"\nkeymap = \"standard\"\n",
    )
    .expect("config file");

    let loaded = load_config(Some(&path));
    let resolved = resolve_user_config(&loaded, &InvocationOverrides::new());

    assert_eq!(
        resolved.ui().locale().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    assert_eq!(
        resolved.ui().keymap().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    assert_eq!(
        resolved.ui().key_hints().provenance(),
        FieldProvenance::Default
    );
    assert_eq!(
        resolved.show_unavailable_choices().provenance(),
        FieldProvenance::Default
    );
}

#[test]
fn legacy_presence_and_programmatic_explicit_values_are_distinct_from_defaults() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("legacy.toml");
    fs::write(&path, "[ui]\nkeymap = \"standard\"\n").expect("legacy config");
    let legacy = load_config(Some(&path));
    let legacy_resolved = resolve_user_config(&legacy, &InvocationOverrides::new());
    assert_eq!(
        legacy_resolved.ui().keymap().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    assert_eq!(
        legacy_resolved.ui().locale().provenance(),
        FieldProvenance::Default
    );

    let programmatic = recite_config::LoadedUserConfig::from_explicit(UserConfig::default());
    assert_eq!(programmatic.provenance, ConfigProvenance::Programmatic);
    assert_eq!(programmatic.format, ConfigFormat::Defaults);
    assert_eq!(programmatic.path, None);
    let programmatic_resolved = resolve_user_config(&programmatic, &InvocationOverrides::new());
    assert_eq!(
        programmatic_resolved.ui().locale().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
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
