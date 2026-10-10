use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn project(scenes: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        format!("format_version = 1\n[project]\ncontent_set = 'trial'\nversion = '1'\n{scenes}"),
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
    )?;
    Ok(dir)
}

const SCENE: &str = "[[scenes]]\nid = 'start'\nasset = 'build/start.recitec'\nblock = 'start'\nparticipants = ['mara']\n";

#[test]
fn undeclared_or_retired_outputs_fail_before_publishing() -> TestResult {
    let empty = project("")?;
    let outcome = run(empty.path().to_owned(), None, &BuildControl::new());
    assert!(matches!(&outcome, Outcome::Failed(Failure::NoTargets)));
    assert!(outcome.message().contains("Declare a scene and output"));
    let dir = project(SCENE)?;
    let outcome = run(
        dir.path().to_owned(),
        Some("build/retired.recitec".into()),
        &BuildControl::new(),
    );
    assert!(
        matches!(&outcome, Outcome::Failed(Failure::AssetRetired)),
        "{}",
        outcome.message()
    );
    assert!(outcome.message().contains("no longer declared"));
    assert!(!dir.path().join("build").exists());
    Ok(())
}

#[test]
fn preparation_and_source_failures_preserve_previous_output() -> TestResult {
    let dir = project(SCENE)?;
    std::fs::create_dir(dir.path().join("build"))?;
    let asset = dir.path().join("build/start.recitec");
    std::fs::write(&asset, "previous output")?;
    std::fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n-> missing\n",
    )?;
    let outcome = run(dir.path().to_owned(), None, &BuildControl::new());
    assert!(matches!(&outcome, Outcome::Failed(Failure::Diagnostics(_))));
    assert!(!outcome.is_success());
    assert!(outcome.message().contains("missing"));
    assert_eq!(std::fs::read_to_string(&asset)?, "previous output");
    std::fs::remove_file(dir.path().join("recite.project.toml"))?;
    let outcome = run(dir.path().to_owned(), None, &BuildControl::new());
    assert!(matches!(&outcome, Outcome::Failed(Failure::Preparation(_))));
    assert!(!outcome.message().is_empty());
    assert_eq!(std::fs::read_to_string(asset)?, "previous output");
    Ok(())
}

#[test]
fn cancelled_build_retains_previous_output_and_has_an_actionable_status() -> TestResult {
    let dir = project(SCENE)?;
    std::fs::create_dir(dir.path().join("build"))?;
    let asset = dir.path().join("build/start.recitec");
    std::fs::write(&asset, "previous output")?;
    let control = BuildControl::new();
    control.cancel();
    let outcome = run(dir.path().to_owned(), None, &control);
    assert!(
        matches!(&outcome, Outcome::Completed { result, .. } if result.status() == BuildTerminalStatus::Cancelled)
    );
    assert!(outcome.is_success());
    assert!(
        outcome
            .message()
            .contains("Previous outputs remain available")
    );
    assert_eq!(std::fs::read_to_string(asset)?, "previous output");
    Ok(())
}

#[test]
fn disconnected_build_worker_is_reported_as_failure() {
    let (sender, result) = mpsc::sync_channel(1);
    let job = Job {
        control: BuildControl::new(),
        result,
    };
    assert!(job.poll().is_none());
    drop(sender);
    let outcome = job.poll();
    assert!(matches!(
        outcome,
        Some(Outcome::Failed(Failure::WorkerStopped))
    ));
}
