use super::*;

fn finish(job: &Handoff) -> Result<Result<(), String>, String> {
    for _ in 0..1000 {
        if let Some(result) = job.poll() {
            return Ok(result);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Err("external application did not finish".into())
}

#[cfg(unix)]
#[test]
fn explicit_editor_command_reports_success_and_nonzero_exit_status() -> Result<(), String> {
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", "exit 0"]);
    finish(&Handoff::command(command)?)??;
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", "exit 7"]);
    let error = finish(&Handoff::command(command)?)?.unwrap_err();
    assert!(error.contains("7"));
    assert!(error.contains("Check the configured editor command"));
    Ok(())
}

#[test]
fn absent_editor_program_and_disconnected_launcher_report_failure() -> Result<(), String> {
    let command = std::process::Command::new("/recite-test-missing/editor");
    assert!(finish(&Handoff::command(command)?)?.is_err());
    let (sender, result) = mpsc::sync_channel(1);
    let job = Handoff { result };
    assert!(job.poll().is_none());
    drop(sender);
    assert_eq!(
        job.poll(),
        Some(Err(
            "The external application launcher stopped unexpectedly.".into()
        ))
    );
    Ok(())
}
