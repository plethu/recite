use crate::{Result, version};
use std::{fs, process::Command};

pub(super) fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(super) fn candidate(version: &str, commit: &str) -> Result<()> {
    version::parse(version)?;
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("candidate commit must be a complete Git object ID".into());
    }
    if git(&["rev-parse", "HEAD"])? != commit {
        return Err("candidate checkout does not match the requested commit".into());
    }
    if !git(&["status", "--porcelain"])?.is_empty() {
        return Err("candidate checkout has uncommitted changes".into());
    }
    if workspace_version("Cargo.toml")? != version {
        return Err("prepare the product version before building a candidate".into());
    }
    let packaging: serde_json::Value =
        serde_json::from_slice(&fs::read("apps/writer/packaging/common.json")?)?;
    if packaging["version"].as_str() != Some(workspace_version("apps/writer/Cargo.toml")?.as_str())
    {
        return Err("Writer package version differs from the candidate".into());
    }
    Ok(())
}

pub(super) fn workspace_version(manifest: &str) -> Result<String> {
    let value: toml::Value = toml::from_str(&fs::read_to_string(manifest)?)?;
    value
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{manifest}: missing workspace version").into())
}

pub(super) fn tag(version: &str, commit: &str) -> Result<()> {
    version::parse(version)?;
    let tag = format!("v{version}");
    if git(&["cat-file", "-t", &tag])? != "tag" {
        return Err(
            "release tags must be annotated; verify their signature before publication".into(),
        );
    }
    if git(&["rev-parse", &format!("{tag}^{{commit}}")])? != commit {
        return Err("signed release tag does not identify the verified candidate".into());
    }
    git(&["merge-base", "--is-ancestor", commit, "origin/main"])?;
    Ok(())
}
