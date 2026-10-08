use crate::Result;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

fn inventory(root: &Path, directory: &Path, files: &mut BTreeMap<String, String>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("candidate artifacts cannot contain symbolic links".into());
        }
        let path = entry.path();
        if kind.is_dir() {
            inventory(root, &path, files)?;
        } else if kind.is_file() {
            let mut source = fs::File::open(&path)?;
            let mut digest = Sha256::new();
            let mut buffer = [0; 65536];
            loop {
                let count = source.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                digest.update(&buffer[..count]);
            }
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or("artifact path is not UTF-8")?
                .replace('\\', "/");
            files.insert(relative, format!("{:x}", digest.finalize()));
        } else {
            return Err("candidate artifact is not a regular file or directory".into());
        }
    }
    Ok(())
}

pub(super) fn files(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    inventory(root, root, &mut files)?;
    if files.is_empty() {
        return Err("candidate has no artifacts".into());
    }
    Ok(files)
}
