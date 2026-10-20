use std::process::Command;

#[test]
fn help_and_startup_errors_run_without_a_display_or_touching_a_project() {
    for flag in ["--help", "-h"] {
        let output = Command::new(env!("CARGO_BIN_EXE_recite-writer-xilem"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("--project PATH")
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let environment = tempfile::tempdir().unwrap();
    for arguments in [
        vec![],
        vec!["--unknown", "value"],
        vec!["--project", directory.path().to_str().unwrap()],
        vec![
            "--project",
            directory.path().to_str().unwrap(),
            "--editor",
            "unused-editor",
            "--editor-arg",
            "--help",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_recite-writer-xilem"))
            .args(arguments)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WINIT_UNIX_BACKEND")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env_remove("RECITE_CONFIG")
            .env("XDG_RUNTIME_DIR", environment.path())
            .env("XDG_CONFIG_HOME", environment.path().join("config"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
        assert!(
            output.stdout.is_empty(),
            "an editor argument must not invoke Writer help"
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
