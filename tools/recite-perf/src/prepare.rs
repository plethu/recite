//! Integrate the current harness without replacing the control's dependency declarations.
use std::fs;
use std::path::Path;

use toml_edit::{Array, DocumentMut, Item};

pub(super) fn workspace(
    candidate: &Path,
    control: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let current = fs::read_to_string(candidate.join("Cargo.toml"))?.parse::<DocumentMut>()?;
    let mut manifest = fs::read_to_string(control.join("Cargo.toml"))?.parse::<DocumentMut>()?;
    for (key, added) in [
        ("members", &["tools/recite-perf"][..]),
        ("exclude", &["apps/writer", "apps/writer/crates/*"][..]),
    ] {
        let field = &mut manifest["workspace"][key];
        if field.is_none() {
            *field = Item::Value(Array::new().into());
        }
        let array = field.as_array_mut().ok_or("invalid workspace path list")?;
        for path in added {
            if !array.iter().any(|value| value.as_str() == Some(path)) {
                array.push(*path);
            }
        }
    }
    if manifest["workspace"]["dependencies"]["toml_edit"].is_none() {
        manifest["workspace"]["dependencies"]["toml_edit"] =
            current["workspace"]["dependencies"]["toml_edit"].clone();
    }
    fs::write(control.join("Cargo.toml"), manifest.to_string())?;
    Ok(())
}
