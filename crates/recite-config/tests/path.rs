#![expect(
    clippy::expect_used,
    reason = "path integration tests fail fast on platform-root and temporary-directory fixture setup; standalone test targets are outside clippy.toml's test allowance"
)]

use std::path::Path;

use recite_config::{
    ConfigPathSource, PathResolutionError, Platform, PlatformRoots, ResolvedConfigPath,
    resolve_config_path,
};

fn resolved_path(
    platform: Platform,
    roots: &PlatformRoots,
    explicit_override: Option<&Path>,
) -> ResolvedConfigPath {
    resolve_config_path(platform, roots, explicit_override)
        .expect("valid test path roots")
        .expect("test path resolution")
}

#[test]
fn linux_prefers_xdg_over_home_and_uses_recite_file() {
    let roots = PlatformRoots::new()
        .with_xdg_config_home("/synthetic/xdg")
        .with_home("/synthetic/home");

    let resolved = resolved_path(Platform::Linux, &roots, None);

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
    let resolved = resolved_path(Platform::Linux, &roots, None);
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
    let mac = resolved_path(
        Platform::MacOs,
        &PlatformRoots::new().with_application_support("/synthetic/library/Application Support"),
        None,
    );
    assert_eq!(
        mac.path(),
        Path::new("/synthetic/library/Application Support/Recite/config.toml")
    );

    let windows = resolved_path(
        Platform::Windows,
        &PlatformRoots::new().with_roaming_app_data("/synthetic/appdata/roaming"),
        None,
    );
    assert_eq!(
        windows.path(),
        Path::new("/synthetic/appdata/roaming/Recite/config.toml")
    );
}

#[test]
fn explicit_override_is_absolute_and_has_precedence() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let explicit = directory.path().join("explicit.toml");
    let roots = PlatformRoots::new().with_home("/synthetic/home");
    let resolved = resolved_path(Platform::Linux, &roots, Some(&explicit));

    assert_eq!(resolved.path(), explicit);
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
