use std::{fs, path::Path};

use freya::prelude::*;
use freya_testing::prelude::*;
use recite_writer::Startup;

fn project(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(root)?;
    fs::write(
        root.join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    fs::write(root.join("scene.recite"), ":: arrival default\n-> END\n")?;
    Ok(())
}

#[test]
fn desktop_url_selects_its_project_and_preserves_encoded_route()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("Écriture & notes");
    project(&root)?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("project", &root.to_string_lossy())
        .append_pair("scene", "scene.recite")
        .append_pair("q", "café & tea")
        .append_pair("page", "2")
        .finish();
    let link = format!("recite://writer/translations?{query}");
    assert_eq!(
        Startup::parse([link.clone()])?,
        Startup::Writer {
            project: Some(root),
            route: Some(link),
        }
    );
    Ok(())
}

#[test]
fn explicit_and_linked_projects_must_resolve_to_the_same_root()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    project(&first)?;
    project(&second)?;
    let link = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &second.to_string_lossy())
            .finish()
    );
    assert!(
        Startup::parse([
            "--project".into(),
            first.to_string_lossy().into_owned(),
            "--route".into(),
            link.clone(),
        ])
        .unwrap_err()
        .contains("another project")
    );
    assert_eq!(
        Startup::parse([
            "--project".into(),
            second.join("scene.recite").to_string_lossy().into_owned(),
            "--route".into(),
            link.clone(),
        ])?,
        Startup::Writer {
            project: Some(second),
            route: Some(link),
        }
    );
    Ok(())
}

#[test]
fn startup_modes_and_invalid_arguments_are_explicit() -> Result<(), String> {
    assert_eq!(
        Startup::parse([])?,
        Startup::Writer {
            project: None,
            route: None
        }
    );
    assert_eq!(
        Startup::parse(["--examples".into()])?,
        Startup::Examples { route: None }
    );
    assert_eq!(
        Startup::parse(["--design-system".into()])?,
        Startup::DesignSystem
    );
    assert_eq!(Startup::parse(["--help".into()])?, Startup::Help);
    assert_eq!(Startup::parse(["--version".into()])?, Startup::Version);
    for args in [
        vec!["--project"],
        vec!["--route"],
        vec!["--route", "recite://other/write"],
        vec!["--route", "recite://writer/write"],
        vec!["--route", "recite://writer/write", "recite://writer/write"],
        vec!["--examples", "--project", "any"],
        vec!["--design-system", "--route", "recite://writer/write"],
        vec!["--unknown"],
    ] {
        assert!(Startup::parse(args.into_iter().map(str::to_owned)).is_err());
    }
    Ok(())
}

#[test]
fn launch_argument_conflicts_report_the_action_the_user_can_correct() -> Result<(), String> {
    for (args, message) in [
        (
            vec!["--project", "one", "--project", "two"],
            "--project may only be supplied once",
        ),
        (
            vec![
                "--route",
                "recite://writer/write",
                "--route",
                "recite://writer/write",
            ],
            "A writer route may only be supplied once",
        ),
        (vec!["--project", "--examples"], "--project needs a path"),
        (vec!["--route", "--examples"], "--route needs a URL"),
        (vec!["--project", ""], "--project needs a path"),
        (vec!["--route", ""], "--route needs a URL"),
        (
            vec!["--help", "--version"],
            "Choose either --help or --version",
        ),
        (
            vec!["--design-system", "--examples"],
            "--design-system cannot open a project or route",
        ),
    ] {
        assert_eq!(
            Startup::parse(args.into_iter().map(str::to_owned)),
            Err(message.into())
        );
    }
    assert_eq!(Startup::parse(["-h".into()])?, Startup::Help);
    assert_eq!(Startup::parse(["-V".into()])?, Startup::Version);
    let route = "recite://writer/write?beat=start";
    assert_eq!(
        Startup::parse(["--examples".into(), route.into()])?,
        Startup::Examples {
            route: Some(route.into())
        }
    );
    Ok(())
}

#[test]
fn native_launcher_help_version_and_argument_errors_work_without_a_window()
-> Result<(), Box<dyn std::error::Error>> {
    let binary = env!("CARGO_BIN_EXE_recite-writer");
    let help = std::process::Command::new(binary).arg("--help").output()?;
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)?.starts_with("Usage: recite-writer"));
    assert!(help.stderr.is_empty());
    let version = std::process::Command::new(binary)
        .arg("--version")
        .output()?;
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout)?.trim(),
        format!("recite-writer {}", env!("CARGO_PKG_VERSION"))
    );
    let invalid = std::process::Command::new(binary)
        .arg("--unknown-option")
        .output()?;
    assert!(!invalid.status.success());
    let error = String::from_utf8(invalid.stderr)?;
    assert!(error.contains("Unknown writer argument: --unknown-option"));
    assert!(error.contains("Usage: recite-writer"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn native_launcher_refuses_unsafe_activation_storage_before_opening_a_window()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let config = dir.path().join("configuration");
    fs::create_dir(&config)?;
    fs::set_permissions(&config, fs::Permissions::from_mode(0o777))?;
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_recite-writer"))
        .env("RECITE_CONFIG", config.join("user.toml"))
        .output()?;
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert_eq!(
        String::from_utf8(result.stderr)?.trim(),
        "Could not activate Recite writer: Unsafe user configuration directory"
    );
    assert!(!config.join("writer-activation").exists());
    Ok(())
}

#[test]
fn cold_project_route_opens_the_linked_scene() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("Écriture & notes");
    project(&root)?;
    fs::write(
        root.join("second.recite"),
        ":: other default\n> other@33333333333333333333\n  Other scene.\n-> END\n",
    )?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("project", &root.to_string_lossy())
        .append_pair("scene", "second.recite")
        .append_pair("beat", "other")
        .finish();
    let route = format!("recite://writer/write?{query}");
    let selected = Startup::parse([route.clone()])?;
    let Startup::Writer { project, .. } = selected else {
        return Err("expected writer".into());
    };
    let (mut test, _) = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1400., 1000.),
        move |runner| {
            runner.provide_root_context(move || recite_writer::InitialProject(project));
            runner.provide_root_context(move || recite_writer::InitialRoute(route));
        },
        1.,
    );
    test.poll_n(std::time::Duration::from_millis(16), 50);
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("Other scene.")))
            .is_some()
    );
    Ok(())
}
