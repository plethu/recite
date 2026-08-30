use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) struct StagedOutput {
    pub(super) temp: PathBuf,
    pub(super) output: PathBuf,
}

pub(super) fn stage(output: &Path, bytes: &[u8], generation: u64) -> io::Result<StagedOutput> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let sequence = STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".{file_name}.recite-stage-{generation}-{sequence}.tmp"
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    if let Err(error) = write_and_sync(&mut file, bytes) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(StagedOutput {
        temp,
        output: output.to_owned(),
    })
}

pub(super) fn remove(staged: &StagedOutput) {
    let _ = fs::remove_file(&staged.temp);
}

pub(super) fn replace(staged: &StagedOutput) -> io::Result<()> {
    fs::rename(&staged.temp, &staged.output)
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}
