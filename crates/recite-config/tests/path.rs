#![allow(clippy::expect_used)]

use std::path::Path;

use recite_config::{
    ConfigPathSource, PathResolutionError, Platform, PlatformRoots, resolve_config_path,
};

#[test]
fn linux_prefers_xdg_over_home_and_uses_recite_file() {
    let roots = PlatformRoots::new()
        .with_xdg_config_home("/synthetic/xdg")
        .with_home("/synthetic/home");

    let resolved = resolve_config_path(Platform::Linux, &roots, None)
        .expect("valid roots")
        .expect("xdg root");

    assert_eq!(
        resolved.path(),
        Path::new("/synthetic/xdg/recite/config.toml")
    );
    assert_eq!(resolved.source(), ConfigPathSource::PlatformDefault);
    assert!(!resolved.is_explicit());
}

#[test]
fn linux_home_fallback_and_missing_default_are_explicit() {
    let roots = PlatformRoots::new().with_home("/synthetic/home");
    let resolved = resolve_config_path(Platform::Linux, &roots, None)
        .expect("valid roots")
        .expect("home fallback");
    assert_eq!(
        resolved.path(),
        Path::new("/synthetic/home/.config/recite/config.toml")
    );

    assert!(
        resolve_config_path(Platform::Linux, &PlatformRoots::new(), None)
            .expect("missing default roots are defaults")
            .is_none()
    );
}

#[test]
fn macos_and_windows_use_roaming_platform_locations() {
    let mac = resolve_config_path(
        Platform::MacOs,
        &PlatformRoots::new().with_application_support("/synthetic/library/Application Support"),
        None,
    )
    .expect("valid macOS roots")
    .expect("Application Support root");
    assert_eq!(
        mac.path(),
        Path::new("/synthetic/library/Application Support/Recite/config.toml")
    );

    let windows = resolve_config_path(
        Platform::Windows,
        &PlatformRoots::new().with_roaming_app_data("/synthetic/appdata/roaming"),
        None,
    )
    .expect("valid Windows roots")
    .expect("roaming AppData root");
    assert_eq!(
        windows.path(),
        Path::new("/synthetic/appdata/roaming/Recite/config.toml")
    );
}

#[test]
fn explicit_override_is_absolute_and_has_precedence() {
    let roots = PlatformRoots::new().with_home("/synthetic/home");
    let resolved = resolve_config_path(
        Platform::Linux,
        &roots,
        Some(Path::new("/synthetic/explicit.toml")),
    )
    .expect("absolute explicit path")
    .expect("explicit path");

    assert_eq!(resolved.path(), Path::new("/synthetic/explicit.toml"));
    assert_eq!(resolved.source(), ConfigPathSource::ExplicitOverride);
    assert!(resolved.is_explicit());
}

#[test]
fn empty_and_relative_explicit_overrides_fail_typed() {
    assert_eq!(
        resolve_config_path(Platform::Linux, &PlatformRoots::new(), Some(Path::new(""))),
        Err(PathResolutionError::EmptyExplicitOverride)
    );
    assert_eq!(
        resolve_config_path(
            Platform::Linux,
            &PlatformRoots::new(),
            Some(Path::new("config.toml")),
        ),
        Err(PathResolutionError::RelativeExplicitOverride {
            path: "config.toml".into()
        })
    );
}
