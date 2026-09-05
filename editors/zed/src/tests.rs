use super::launcher::{LauncherSettings, resolve};

#[test]
fn configured_path_preserves_arguments_and_sorts_environment() {
    let command = resolve(
        LauncherSettings {
            path: Some("/opt/recite-lsp".into()),
            arguments: vec!["--stdio".into(), "--verbose".into()],
            environment: vec![
                ("Z_LAST".into(), "3".into()),
                ("A_FIRST".into(), "1".into()),
            ],
        },
        Some("/ignored/recite-lsp".into()),
    )
    .expect("configured path should win");

    assert_eq!(command.command, "/opt/recite-lsp");
    assert_eq!(command.arguments, ["--stdio", "--verbose"]);
    assert_eq!(
        command.environment,
        vec![
            ("A_FIRST".to_string(), "1".to_string()),
            ("Z_LAST".to_string(), "3".to_string())
        ]
    );
}

#[test]
fn path_fallback_carries_configured_arguments_and_environment() {
    let command = resolve(
        LauncherSettings {
            path: None,
            arguments: vec!["--config".into(), "project.toml".into()],
            environment: vec![
                ("RECITE_Z".into(), "z".into()),
                ("RECITE_A".into(), "a".into()),
            ],
        },
        Some("/usr/local/bin/recite-lsp".into()),
    )
    .expect("PATH fallback should resolve");

    assert_eq!(command.command, "/usr/local/bin/recite-lsp");
    assert_eq!(command.arguments, ["--config", "project.toml"]);
    assert_eq!(
        command.environment,
        vec![
            ("RECITE_A".to_string(), "a".to_string()),
            ("RECITE_Z".to_string(), "z".to_string())
        ]
    );
}

#[test]
fn blank_configured_path_is_refused_without_fallback() {
    let error = resolve(
        LauncherSettings {
            path: Some("  ".into()),
            ..LauncherSettings::default()
        },
        Some("/usr/bin/recite-lsp".into()),
    )
    .expect_err("blank configured path must be actionable configuration error");

    assert!(error.contains("binary.path"));
    assert!(error.contains("PATH"));
}

#[test]
fn missing_binary_error_names_install_and_configuration() {
    let error = resolve(LauncherSettings::default(), None)
        .expect_err("missing PATH binary must be actionable");

    assert!(error.contains("Install it separately"));
    assert!(error.contains("binary.path"));
}

#[test]
fn equal_environment_keys_retain_input_order() {
    let command = resolve(
        LauncherSettings {
            environment: vec![
                ("DUPLICATE".into(), "first".into()),
                ("DUPLICATE".into(), "second".into()),
            ],
            ..LauncherSettings::default()
        },
        Some("recite-lsp".into()),
    )
    .expect("fallback should resolve");

    assert_eq!(
        command.environment,
        vec![
            ("DUPLICATE".to_string(), "first".to_string()),
            ("DUPLICATE".to_string(), "second".to_string()),
        ]
    );
}
