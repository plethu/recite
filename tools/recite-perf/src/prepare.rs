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
    adapt_lsp_driver(control)?;
    Ok(())
}

// Keep the current harness on the historical diagnostic-publication path when
// that revision predates the related-source fast path; never overlay its owner.
fn adapt_lsp_driver(control: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = control.join("crates/recite-lsp/src/bench_support/driver.rs");
    if !path.is_file() {
        return Ok(());
    }
    let owner = fs::read_to_string(control.join("crates/recite-lsp/src/workspace/ui.rs"))?;
    if owner.contains("fn diagnostic_related_sources(") {
        return Ok(());
    }
    let source = fs::read_to_string(&path)?;
    let current = ".diagnostic_related_sources(&diagnostics.uri, &diagnostics.diagnostics)";
    let historical = ".diagnostic_sources_for_uri(&diagnostics.uri)";
    match source.matches(current).count() {
        0 if source.matches(historical).count() == 1 => Ok(()),
        1 => {
            fs::write(path, source.replace(current, historical))?;
            Ok(())
        }
        _ => Err("unrecognized control LSP diagnostic publication adapter".into()),
    }
}
