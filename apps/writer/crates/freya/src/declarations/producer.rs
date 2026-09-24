//! Explicit producer processes write staged output; only validated results reach publication.
use recite_config::{ProducerCommand, ProducerRegistration};
use recite_core::schema::ProjectSchema;
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use wait_timeout::ChildExt;

pub(super) struct Generated {
    pub text: String,
    pub schema: ProjectSchema,
}
pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<Generated, String>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Job {
    pub fn start(root: PathBuf, registration: ProducerRegistration) -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (send, result) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("recite-schema-producer".into())
            .spawn(move || {
                let _ = send.send(generate(&root, &registration, &flag));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { cancel, result })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn poll(&self) -> Option<Result<Generated, String>> {
        match self.result.try_recv() {
            Ok(value) => Some(if self.cancel.load(Ordering::Relaxed) {
                Err("Generation cancelled. Previous schema retained.".into())
            } else {
                value
            }),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "The schema producer worker stopped unexpectedly.".into(),
            )),
        }
    }
}
pub(super) fn command(
    root: &Path,
    spec: &ProducerCommand,
    replacements: &[(&str, String)],
) -> Result<Command, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let directory = root
        .join(spec.directory())
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !directory.starts_with(&root) {
        return Err("The producer working directory resolves outside this project.".into());
    }
    let program = Path::new(spec.program());
    let program = if program.is_relative() && program.components().count() > 1 {
        directory.join(program)
    } else {
        program.to_owned()
    };
    let in_flatpak = cfg!(target_os = "linux") && Path::new("/.flatpak-info").is_file();
    let mut command = configured_process(&program, &directory, in_flatpak);
    for arg in spec.args() {
        let mut expanded = arg.clone();
        for (key, value) in replacements {
            expanded = expanded.replace(key, value);
        }
        command.arg(expanded);
    }
    Ok(command)
}

fn configured_process(program: &Path, directory: &Path, in_flatpak: bool) -> Command {
    let mut command = if in_flatpak {
        // Only explicitly configured producer/editor commands use the host.
        // WATCH_BUS also terminates the host child when cancellation kills this proxy.
        let mut command = Command::new("/usr/bin/flatpak-spawn");
        let mut working_directory = std::ffi::OsString::from("--directory=");
        working_directory.push(directory);
        command
            .args(["--host", "--watch-bus"])
            .arg(working_directory)
            .arg("--")
            .arg(program);
        command
    } else {
        Command::new(program)
    };
    command.current_dir(directory).stdin(Stdio::null());
    command
}
fn generate(
    root: &Path,
    registration: &ProducerRegistration,
    cancel: &AtomicBool,
) -> Result<Generated, String> {
    let stage = tempfile::Builder::new()
        .prefix(".recite-schema-stage-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    let output = stage.path().join("schema.json");
    let log_path = stage.path().join("producer.log");
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let mut command = command(
        root,
        registration.generate(),
        &[("{output}", output.to_string_lossy().into_owned())],
    )?;
    command
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", registration.generate().program()))?;
    let mut wait_budget = Duration::from_secs(120);
    let poll = Duration::from_millis(50);
    let status = loop {
        if cancel.load(Ordering::Relaxed) || wait_budget.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Generation cancelled or exceeded two minutes. The previous schema remains available.".into());
        }
        match child.wait_timeout(poll) {
            Ok(Some(status)) => break status,
            Ok(None) => wait_budget = wait_budget.saturating_sub(poll),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    };
    if !status.success() {
        let mut log = Vec::new();
        std::fs::File::open(log_path)
            .map_err(|e| e.to_string())?
            .take(64 * 1024)
            .read_to_end(&mut log)
            .map_err(|e| e.to_string())?;
        return Err(format!(
            "Producer exited with {status}. Previous schema retained.\n{}",
            String::from_utf8_lossy(&log)
        ));
    }
    let text = crate::project::read_regular(&output)
        .map_err(|e| format!("Producer did not supply a regular schema output: {e}"))?;
    let loaded = recite_core::schema::load_schema_manifest_str("producer output", &text);
    let schema = loaded
        .schema
        .ok_or_else(|| crate::project_context::diagnostic_messages(&loaded.diagnostics))?;
    if schema
        .producer_metadata
        .as_ref()
        .and_then(|m| m.producer.as_ref())
        != Some(registration.producer())
    {
        return Err(
            "Generated schema belongs to a different producer. Previous schema retained.".into(),
        );
    }
    Ok(Generated { text, schema })
}
#[cfg(test)]
mod tests;
