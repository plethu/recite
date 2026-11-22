//! Running a registered producer explicitly: its configured command writes a staged schema, which
//! is validated and returned without touching the project's live schema. Callers decide whether to
//! publish it (Writer's Regenerate) or only compare it (`recite schema check-fresh`).
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use recite_core::{Diagnostic, schema::ProjectSchema};
use wait_timeout::ChildExt;

use super::{ProducerCommand, ProducerRegistration};

/// How long a producer may run before it is stopped.
pub const PRODUCER_DEADLINE: Duration = Duration::from_secs(120);
/// How much producer output a failure report keeps.
const LOG_LIMIT: u64 = 64 * 1024;

/// A validated schema export from the registered producer.
#[derive(Debug)]
pub struct GeneratedSchema {
    pub text: String,
    pub schema: ProjectSchema,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerRunError {
    #[error("the producer working directory resolves outside this project")]
    DirectoryOutsideProject,
    #[error("could not prepare the producer: {0}")]
    Prepare(std::io::Error),
    #[error("could not start {program}: {source}")]
    Start {
        program: String,
        source: std::io::Error,
    },
    #[error("generation was cancelled or exceeded its deadline")]
    Stopped,
    #[error("the producer exited with {status}\n{log}")]
    Failed { status: ExitStatus, log: String },
    #[error("the producer did not supply a regular schema output: {0}")]
    Output(std::io::Error),
    #[error("the producer's schema is invalid")]
    Invalid(Vec<Diagnostic>),
    #[error("the generated schema belongs to a different producer")]
    WrongProducer,
}

/// The configured command, run from its project-relative directory with `{name}` placeholders
/// replaced. Arguments are argv entries; nothing is passed through a shell.
pub fn producer_command(
    root: &Path,
    spec: &ProducerCommand,
    replacements: &[(&str, String)],
) -> Result<Command, ProducerRunError> {
    let root = root.canonicalize().map_err(ProducerRunError::Prepare)?;
    let directory = root
        .join(spec.directory())
        .canonicalize()
        .map_err(ProducerRunError::Prepare)?;
    if !directory.starts_with(&root) {
        return Err(ProducerRunError::DirectoryOutsideProject);
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

/// The process for a configured tool. Inside a Flatpak sandbox only explicitly configured
/// producer and editor commands run on the host; `--watch-bus` ends the host child when the proxy
/// is killed.
#[must_use]
pub fn configured_process(program: &Path, directory: &Path, in_flatpak: bool) -> Command {
    let mut command = if in_flatpak {
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

/// Run the registered producer into a staged output inside the project, wait up to `deadline` or
/// until `cancel` is set, and return the validated export. The live schema is never written.
pub fn generate_schema(
    root: &Path,
    registration: &ProducerRegistration,
    cancel: &AtomicBool,
    deadline: Duration,
) -> Result<GeneratedSchema, ProducerRunError> {
    let stage = tempfile::Builder::new()
        .prefix(".recite-schema-stage-")
        .tempdir_in(root)
        .map_err(ProducerRunError::Prepare)?;
    let output = stage.path().join("schema.json");
    let log_path = stage.path().join("producer.log");
    let log = std::fs::File::create(&log_path).map_err(ProducerRunError::Prepare)?;
    let mut command = producer_command(
        root,
        registration.generate(),
        &[("{output}", output.to_string_lossy().into_owned())],
    )?;
    command
        .stdout(log.try_clone().map_err(ProducerRunError::Prepare)?)
        .stderr(log);
    let mut child = command.spawn().map_err(|source| ProducerRunError::Start {
        program: registration.generate().program().to_owned(),
        source,
    })?;
    let mut budget = deadline;
    let poll = Duration::from_millis(50);
    let status = loop {
        if cancel.load(Ordering::Relaxed) || budget.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProducerRunError::Stopped);
        }
        match child.wait_timeout(poll) {
            Ok(Some(status)) => break status,
            Ok(None) => budget = budget.saturating_sub(poll),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProducerRunError::Prepare(error));
            }
        }
    };
    if !status.success() {
        return Err(ProducerRunError::Failed {
            status,
            log: read_log(&log_path),
        });
    }
    let text = read_regular(&output).map_err(ProducerRunError::Output)?;
    let loaded = recite_core::schema::load_schema_manifest_str("producer output", &text);
    let schema = loaded
        .schema
        .ok_or(ProducerRunError::Invalid(loaded.diagnostics))?;
    if schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.as_ref())
        != Some(registration.producer())
    {
        return Err(ProducerRunError::WrongProducer);
    }
    Ok(GeneratedSchema { text, schema })
}

fn read_log(path: &PathBuf) -> String {
    let mut log = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(LOG_LIMIT).read_to_end(&mut log);
    }
    String::from_utf8_lossy(&log).into_owned()
}

fn read_regular(path: &Path) -> std::io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a regular file",
        ));
    }
    std::fs::read_to_string(path)
}
