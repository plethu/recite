use std::fs;
use std::path::{Path, PathBuf};

#[cfg(any(windows, target_os = "macos"))]
use unicase::UniCase;
#[cfg(any(windows, target_os = "macos"))]
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
pub(super) struct PhysicalIdentity {
    pub(super) canonical: PathBuf,
    #[cfg(any(windows, target_os = "macos"))]
    comparison_key: Vec<UniCase<String>>,
}

pub(super) fn physical_identity(path: &Path) -> Result<PhysicalIdentity, String> {
    let canonical = if path.exists() {
        fs::canonicalize(path).map_err(|error| error.to_string())?
    } else {
        let file_name = path
            .file_name()
            .ok_or_else(|| "target path has no file name".to_owned())?;
        let parent = path
            .parent()
            .ok_or_else(|| "target path has no parent".to_owned())?;
        fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .unwrap_or_else(|_| path.to_owned())
    };
    Ok(PhysicalIdentity {
        #[cfg(any(windows, target_os = "macos"))]
        comparison_key: comparison_key(&canonical),
        canonical,
    })
}

pub(super) fn same_physical_path(left: &PhysicalIdentity, right: &PhysicalIdentity) -> bool {
    if left.canonical == right.canonical {
        return true;
    }
    #[cfg(any(windows, target_os = "macos"))]
    {
        left.comparison_key == right.comparison_key
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        false
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn comparison_key(path: &Path) -> Vec<UniCase<String>> {
    path.components()
        .map(|component| {
            let normalized = component
                .as_os_str()
                .to_string_lossy()
                .nfkc()
                .collect::<String>();
            UniCase::new(normalized)
        })
        .collect()
}
