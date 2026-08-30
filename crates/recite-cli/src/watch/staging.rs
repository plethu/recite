use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{hash::Hash, hash::Hasher};

use recite_compiler::BuildRequest;

static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) struct StagedOutput {
    pub(super) temp: PathBuf,
    pub(super) output: PathBuf,
}

pub(super) fn stage(
    output: &Path,
    bytes: &[u8],
    request: &BuildRequest,
) -> io::Result<StagedOutput> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let request_tag = request_tag(request);
    for _ in 0..128 {
        let sequence = STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{file_name}.recite-stage-{}-{request_tag:016x}-{sequence}.tmp",
            std::process::id()
        ));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = write_and_sync(&mut file, bytes) {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
        return Ok(StagedOutput {
            temp,
            output: output.to_owned(),
        });
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique Recite stage marker",
    ))
}

pub(super) fn remove(staged: &StagedOutput) -> io::Result<()> {
    match fs::remove_file(&staged.temp) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) enum ReplaceOutcome {
    Failed,
    Committed,
    CommittedWithCleanup(io::Error),
}

pub(super) fn replace(staged: &StagedOutput) -> ReplaceOutcome {
    let bytes = match fs::read(&staged.temp) {
        Ok(bytes) => bytes,
        Err(_) => return ReplaceOutcome::Failed,
    };
    let mut replacement = match atomic_write_file::AtomicWriteFile::options().open(&staged.output) {
        Ok(file) => file,
        Err(_) => return ReplaceOutcome::Failed,
    };
    if replacement.as_file_mut().write_all(&bytes).is_err() {
        return ReplaceOutcome::Failed;
    }
    if replacement.commit().is_err() {
        return ReplaceOutcome::Failed;
    }
    match fs::remove_file(&staged.temp) {
        Ok(()) => ReplaceOutcome::Committed,
        Err(error) if error.kind() == io::ErrorKind::NotFound => ReplaceOutcome::Committed,
        Err(error) => ReplaceOutcome::CommittedWithCleanup(error),
    }
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}

fn request_tag(request: &BuildRequest) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{request:?}").hash(&mut hasher);
    hasher.finish()
}
