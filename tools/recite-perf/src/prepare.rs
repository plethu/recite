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
    adapt_completion_workload(control)?;
    adapt_session_snapshots(control)?;
    Ok(())
}

// Session snapshots took their asset and gained self-contained effect requests
// with cross-revision resume (#323); earlier controls encode from the session.
fn adapt_session_snapshots(control: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let harness = control.join("crates/recite-benchmarks/src");
    if !harness.join("runtime.rs").is_file()
        || control
            .join("crates/recite-runtime/src/resume.rs")
            .is_file()
    {
        return Ok(());
    }
    rewrite_harness(
        &harness.join("runtime.rs"),
        &[
            (
                "encode_session_messagepack(self.asset, &session)",
                "encode_session_messagepack(&session)",
                1,
            ),
            (
                "encode_session_messagepack(self.asset, session)",
                "encode_session_messagepack(session)",
                1,
            ),
        ],
        "session snapshot encoding",
    )?;
    rewrite_harness(
        &harness.join("preview_hash_dialogue_errors.rs"),
        &[(
            INVALID_EFFECT_REQUEST_ARM,
            "        // Snapshot effect requests predate this control.\n",
            1,
        )],
        "snapshot conversion errors",
    )
}

const INVALID_EFFECT_REQUEST_ARM: &str = "        \
recite_runtime::snapshot::DialogueSessionSnapshotConversionError::InvalidEffectRequest {
            id,
            reason,
        } => {
            tag(hasher, 2);
            hash_text(hasher, id);
            hash_text(hasher, reason);
        }
";

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
    rewrite_harness(
        &path,
        &[(
            ".diagnostic_related_sources(&diagnostics.uri, &diagnostics.diagnostics)",
            ".diagnostic_sources_for_uri(&diagnostics.uri)",
            1,
        )],
        "LSP diagnostic publication",
    )
}

fn adapt_completion_workload(control: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = control.join("tools/recite-perf/benches/support/completion.rs");
    if !path.is_file() {
        return Ok(());
    }
    let owner = fs::read_to_string(control.join("apps/writer/crates/authoring/src/completion.rs"))?;
    if !owner.contains("pub fn source(&self)") {
        rewrite_harness(
            &path,
            &[(
                "workload.completions.source()",
                "workload.workload.draft.as_str()",
                2,
            )],
            "completion replacement",
        )?;
    }
    // Revisions were plain integers before the typed document version.
    let model = fs::read_to_string(control.join("apps/writer/crates/authoring/src/lib.rs"))?;
    if model.contains("DocumentVersion") {
        return Ok(());
    }
    rewrite_harness(
        &path,
        &[
            (
                "{Document, DocumentVersion, ProjectContext",
                "{Document, ProjectContext",
                1,
            ),
            ("revision: DocumentVersion,", "revision: i64,", 1),
        ],
        "completion revision",
    )
}

/// Replace each exact `(current, historical, occurrences)` snippet. A harness
/// already on every historical snippet is left alone; any other shape fails, so
/// a changed harness cannot silently measure a different path.
fn rewrite_harness(
    path: &Path,
    snippets: &[(&str, &str, usize)],
    adapter: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let on = |historical_side: bool| {
        snippets.iter().all(|&(current, historical, occurrences)| {
            let (wanted, other) = if historical_side {
                (historical, current)
            } else {
                (current, historical)
            };
            source.matches(wanted).count() == occurrences && !source.contains(other)
        })
    };
    if on(true) {
        return Ok(());
    }
    if !on(false) {
        return Err(format!("unrecognized control {adapter} adapter").into());
    }
    let adapted = snippets
        .iter()
        .fold(source, |source, (current, historical, _)| {
            source.replace(current, historical)
        });
    fs::write(path, adapted)?;
    Ok(())
}
