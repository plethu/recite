use crate::{Result, inventory::files, version};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: String,
    commit: String,
    components: BTreeMap<String, String>,
    files: BTreeMap<String, String>,
    assets: BTreeMap<String, String>,
    workflow: Option<Workflow>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Workflow {
    commit: String,
    reference: String,
    run: String,
}

fn validate_targets(plan: &serde_json::Value, releases: &[serde_json::Value]) -> Result<()> {
    let config: toml::Value = toml::from_str(&fs::read_to_string("dist-workspace.toml")?)?;
    let targets: BTreeSet<_> = config
        .get("dist")
        .and_then(|dist| dist.get("targets"))
        .and_then(toml::Value::as_array)
        .ok_or("missing configured distribution targets")?
        .iter()
        .map(|target| target.as_str().ok_or("invalid distribution target"))
        .collect::<std::result::Result<_, _>>()?;
    if targets.is_empty() {
        return Err("distribution target list cannot be empty".into());
    }
    for release in releases {
        let archives: BTreeSet<_> = release["artifacts"]
            .as_array()
            .ok_or("missing release artifacts")?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(|id| &plan["artifacts"][id])
            .filter(|artifact| artifact["kind"] == "executable-zip")
            .flat_map(|artifact| artifact["target_triples"].as_array().into_iter().flatten())
            .filter_map(serde_json::Value::as_str)
            .collect();
        if archives != targets {
            return Err(
                "each CLI/LSP release must cover every configured distribution target".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn seal(
    version: &str,
    commit: &str,
    plan: &Path,
    artifacts: &Path,
    output: &Path,
) -> Result<()> {
    let files = files(artifacts)?;
    let plan: serde_json::Value = serde_json::from_slice(&fs::read(plan)?)?;
    if plan["announcement_tag"].as_str() != Some(&format!("v{version}")) {
        return Err("distribution plan has a different release tag".into());
    }
    let releases = plan["releases"]
        .as_array()
        .ok_or("distribution plan has no releases")?;
    let names: std::collections::BTreeSet<_> = releases
        .iter()
        .filter_map(|release| release["app_name"].as_str())
        .collect();
    if names != ["recite-cli", "recite-lsp"].into_iter().collect() || releases.len() != 2 {
        return Err("distribution plan must contain exactly CLI and LSP releases".into());
    }
    if releases
        .iter()
        .any(|release| release["app_version"].as_str() != Some(version))
    {
        return Err("distribution plan has a different component version".into());
    }
    validate_targets(&plan, releases)?;
    let mut assets = BTreeMap::new();
    for artifact in plan["artifacts"]
        .as_object()
        .ok_or("distribution plan has no artifacts")?
        .values()
    {
        let name = artifact["name"]
            .as_str()
            .ok_or("distribution artifact has no name")?;
        let source = format!("cli/{name}");
        if !files.contains_key(&source) {
            return Err(format!("missing planned distribution artifact: {name}").into());
        }
        assets.insert(name.to_owned(), source);
    }
    for (group, extension) in [
        ("recite-writer-linux-x86_64-preview", "deb"),
        ("recite-writer-macos-arm64-preview", "dmg"),
        ("recite-writer-macos-x86_64-preview", "dmg"),
        ("recite-writer-windows-x86_64-preview", "exe"),
        ("recite-writer-flatpak-x86_64-preview", "flatpak"),
        ("recite-writer-flatpak-aarch64-preview", "flatpak"),
    ] {
        if !files.keys().any(|file| {
            file.starts_with(&format!("writer/{group}/"))
                && Path::new(file).extension().and_then(|value| value.to_str()) == Some(extension)
        }) {
            return Err(format!("missing Writer candidate artifact group: {group}").into());
        }
    }
    for file in files
        .keys()
        .filter(|file| file.starts_with("writer/") && !file.ends_with("/SHA256SUMS"))
    {
        // Writer groups contain repeated metadata and Flatpak basenames. Preserve the
        // platform group in their public filename; dist's installer URLs stay intact.
        let name = file
            .strip_prefix("writer/")
            .ok_or("missing Writer prefix")?
            .replace('/', "-");
        if assets.insert(name, file.clone()).is_some() {
            return Err("public release asset names must be unique".into());
        }
    }
    let manifest = "cli/dist-manifest.json";
    if !files.contains_key(manifest) {
        return Err("missing combined distribution manifest".into());
    }
    assets.insert("dist-manifest.json".into(), manifest.into());
    let ffi: toml::Value = toml::from_str(&fs::read_to_string("crates/recite-ffi/Cargo.toml")?)?;
    let components = BTreeMap::from([
        ("core".into(), version.into()),
        (
            "writer".into(),
            crate::project::workspace_version("apps/writer/Cargo.toml")?,
        ),
        (
            "ffi".into(),
            ffi.get("package")
                .and_then(|value| value.get("version"))
                .and_then(toml::Value::as_str)
                .ok_or("missing FFI crate version")?
                .into(),
        ),
    ]);
    let receipt = Receipt {
        version: version.into(),
        commit: commit.into(),
        components,
        files,
        assets,
        workflow: match std::env::var("GITHUB_RUN_ID") {
            Ok(run) => Some(Workflow {
                commit: std::env::var("GITHUB_SHA")?,
                reference: std::env::var("GITHUB_WORKFLOW_REF")?,
                run,
            }),
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => return Err(error.into()),
        },
    };
    fs::write(output, serde_json::to_string_pretty(&receipt)? + "\n")?;
    Ok(())
}

pub(super) fn assets(
    version: &str,
    commit: &str,
    receipt: &Path,
    artifacts: &Path,
    output: &Path,
) -> Result<()> {
    verify(version, commit, receipt, artifacts)?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(receipt)?)?;
    // A fresh directory prevents leftovers from an earlier publication attempt.
    fs::create_dir(output)?;
    let mut checksums = String::new();
    for (name, source) in receipt.assets {
        if Path::new(&name).file_name().and_then(|part| part.to_str()) != Some(name.as_str())
            || !receipt.files.contains_key(&source)
        {
            return Err("invalid public release asset mapping".into());
        }
        checksums.push_str(&format!("{}  {name}\n", receipt.files[&source]));
        fs::copy(artifacts.join(source), output.join(name))?;
    }
    fs::write(output.join("SHA256SUMS"), checksums)?;
    Ok(())
}

pub(super) fn verify(version: &str, commit: &str, path: &Path, artifacts: &Path) -> Result<()> {
    version::parse(version)?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(path)?)?;
    if receipt.version != version || receipt.commit != commit {
        return Err("release receipt belongs to a different version or commit".into());
    }
    if receipt.files != files(artifacts)? {
        return Err("candidate artifacts differ from the verified receipt".into());
    }
    Ok(())
}

pub(super) fn provenance(path: &Path, run: &str, commit: &str) -> Result<()> {
    let receipt: Receipt = serde_json::from_slice(&fs::read(path)?)?;
    let workflow = receipt
        .workflow
        .ok_or("candidate has no hosted workflow provenance")?;
    if workflow.run != run
        || workflow.commit != commit
        || workflow.reference != "plethu/recite/.github/workflows/release.yml@refs/heads/main"
    {
        return Err("candidate must come from the approved release workflow on main".into());
    }
    Ok(())
}
