use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CliError;

pub(crate) fn write_staged(output: &Path, contents: &[u8]) -> Result<(), CliError> {
    let temp_path = staged_output_path(output);
    if let Err(error) = fs::write(&temp_path, contents) {
        let _ = fs::remove_file(&temp_path);
        return Err(CliError::Write {
            path: temp_path,
            source: error,
        });
    }

    if let Err(error) = fs::rename(&temp_path, output) {
        let _ = fs::remove_file(&temp_path);
        return Err(CliError::Write {
            path: output.to_owned(),
            source: error,
        });
    }

    Ok(())
}

fn staged_output_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("recite-output");
    let temp_name = format!(".{file_name}.{}.tmp", std::process::id());

    match output.parent() {
        Some(parent) => parent.join(temp_name),
        None => PathBuf::from(temp_name),
    }
}
