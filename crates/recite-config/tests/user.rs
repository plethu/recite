#![expect(
    clippy::expect_used,
    reason = "config integration tests fail fast on temporary-file and parsed-fixture setup; standalone test targets are outside clippy.toml's test allowance"
)]

use std::fs;

use recite_config::{
    CONFIG_VERSION, ConfigDiagnostic, ConfigFormat, ConfigProvenance, KeyHints, Keymap, Platform,
    PlatformRoots, TuiColorMode, TuiContrast, UiLocale, load_user_config_from,
};
use tempfile::tempdir;

fn roots_for(directory: &std::path::Path) -> PlatformRoots {
    PlatformRoots::new().with_xdg_config_home(directory)
}

fn config_path(directory: &std::path::Path) -> std::path::PathBuf {
    directory.join("recite/config.toml")
}

fn locale(value: &str) -> UiLocale {
    UiLocale::parse(value).expect("test locale fixture is valid")
}

#[test]
fn absent_platform_default_returns_defaults_without_a_file() {
    let directory = tempdir().expect("temporary directory");
    let loaded = load_user_config_from(Platform::Linux, &roots_for(directory.path()), None)
        .expect("absent default is valid");

    assert_eq!(loaded.provenance, ConfigProvenance::PlatformDefault);
    assert_eq!(loaded.format, ConfigFormat::Defaults);
    assert_eq!(loaded.config.config_version, CONFIG_VERSION);
    assert!(loaded.config.play.show_unavailable_choices);
}

#[test]
fn versioned_user_config_is_strict_and_typed() {
    let directory = tempdir().expect("temporary directory");
    fs::create_dir_all(directory.path().join("recite")).expect("config directory");
    fs::write(
        config_path(directory.path()),
        r#"
config_version = 1

[ui]
locale = "fr-CA"
keymap = "vim"
key_hints = "compact"
color = "never"
contrast = "accessible"

[play]
show_unavailable_choices = false
"#,
    )
    .expect("config file");

    let loaded = load_user_config_from(Platform::Linux, &roots_for(directory.path()), None)
        .expect("version 1 config");
    assert_eq!(loaded.provenance, ConfigProvenance::PlatformDefault);
    assert_eq!(loaded.format, ConfigFormat::Versioned);
    assert_eq!(loaded.config.ui.locale, locale("fr-CA"));
    assert_eq!(loaded.config.ui.keymap, Keymap::Vim);
    assert_eq!(loaded.config.ui.key_hints, KeyHints::Compact);
    assert_eq!(loaded.config.ui.color, TuiColorMode::Never);
    assert_eq!(loaded.config.ui.contrast, TuiContrast::Accessible);
    assert!(!loaded.config.play.show_unavailable_choices);
}

#[test]
fn legacy_config_is_read_compatibly_without_being_rewritten() {
    let directory = tempdir().expect("temporary directory");
    fs::create_dir_all(directory.path().join("recite")).expect("config directory");
    let path = config_path(directory.path());
    let source = "[ui]\nlocale = \"system\"\n";
    fs::write(&path, source).expect("legacy config");

    let loaded = load_user_config_from(Platform::Linux, &roots_for(directory.path()), None)
        .expect("legacy config");
    assert_eq!(loaded.format, ConfigFormat::LegacyPreVersioned);
    assert!(loaded.is_legacy());
    assert_eq!(loaded.config.config_version, CONFIG_VERSION);
    assert_eq!(loaded.config.ui.locale, locale("system"));
    assert_eq!(
        fs::read_to_string(path).expect("read unchanged config"),
        source
    );
}

#[test]
fn explicit_missing_and_future_versions_are_typed_failures() {
    let directory = tempdir().expect("temporary directory");
    let missing = directory.path().join("missing.toml");
    let error = load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&missing))
        .expect_err("missing explicit override");
    assert_eq!(
        error.diagnostic(),
        ConfigDiagnostic::MissingExplicitOverride
    );

    let future = directory.path().join("future.toml");
    fs::write(&future, "config_version = 2\n").expect("future config");
    let error = load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&future))
        .expect_err("future version");
    assert_eq!(error.diagnostic(), ConfigDiagnostic::UnsupportedVersion);
    assert!(error.to_string().contains("unsupported version 2"));
}

#[test]
fn malformed_unknown_and_dialogue_locale_fields_do_not_get_merged() {
    let directory = tempdir().expect("temporary directory");
    for (name, source) in [
        (
            "unknown.toml",
            "config_version = 1\n[ui]\nfont = \"serif\"\n",
        ),
        (
            "dialogue.toml",
            "config_version = 1\ndialogue_locale = \"fr-FR\"\n",
        ),
        ("syntax.toml", "config_version = [1]\n"),
    ] {
        let path = directory.path().join(name);
        fs::write(&path, source).expect("malformed config");
        let error = load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&path))
            .expect_err("strict config failure");
        assert_eq!(error.diagnostic(), ConfigDiagnostic::Malformed);
    }
}

#[test]
fn invalid_locale_and_unreadable_explicit_path_have_identity() {
    let directory = tempdir().expect("temporary directory");
    let invalid = directory.path().join("invalid-locale.toml");
    fs::write(
        &invalid,
        "config_version = 1\n[ui]\nlocale = \"not a locale\"\n",
    )
    .expect("invalid locale config");
    let error = load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&invalid))
        .expect_err("invalid locale");
    assert_eq!(error.diagnostic(), ConfigDiagnostic::InvalidLocale);
    assert_eq!(error.as_core_diagnostic().code.as_str(), "RECITE_CONFIG007");

    let error = load_user_config_from(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(directory.path()),
    )
    .expect_err("directory cannot be read as config");
    assert_eq!(error.diagnostic(), ConfigDiagnostic::ReadFailure);
}
