use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use super::model::*;

const MANIFEST_PATH: &str = "fixtures/adapter-conformance/v1/scenarios.json";
const MANIFEST_SCHEMA_PATH: &str =
    "fixtures/adapter-conformance/v1/adapter-conformance-manifest-v1.schema.json";
const OPERATION_SCHEMA_PATH: &str =
    "fixtures/adapter-conformance/v1/adapter-conformance-operation-result-v1.schema.json";
const ADAPTER_CONTRACT_PATH: &str = "docs/engine-adapter-contract.md";

pub(crate) fn load_manifest() -> Result<ConformanceManifest, String> {
    let source = fs::read_to_string(workspace_path(MANIFEST_PATH))
        .map_err(|error| format!("failed to read `{MANIFEST_PATH}`: {error}"))?;
    let manifest: ConformanceManifest = serde_json::from_str(&source)
        .map_err(|error| format!("failed to parse `{MANIFEST_PATH}` JSON: {error}"))?;
    validate_manifest_structure(&manifest)?;
    Ok(manifest)
}

pub(crate) fn load_contract_error_categories() -> Result<Vec<String>, String> {
    let source = fs::read_to_string(workspace_path(ADAPTER_CONTRACT_PATH))
        .map_err(|error| format!("failed to read `{ADAPTER_CONTRACT_PATH}`: {error}"))?;

    let mut in_error_section = false;
    let mut categories = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "## 12. Error Categories" {
            in_error_section = true;
            continue;
        }
        if in_error_section && trimmed.starts_with("## ") {
            break;
        }
        if in_error_section
            && let Some(rest) = trimmed.strip_prefix("- `")
            && let Some(index) = rest.find('`')
        {
            categories.push(rest[..index].to_owned());
        }
    }

    if categories.is_empty() {
        return Err(format!(
            "no stable error categories found in `{ADAPTER_CONTRACT_PATH}` §12"
        ));
    }

    Ok(categories)
}

pub(crate) fn load_manifest_schema_error_categories() -> Result<Vec<String>, String> {
    load_schema_array(
        MANIFEST_SCHEMA_PATH,
        "/properties/stable_error_categories/const",
    )
}

pub(crate) fn load_operation_schema_error_categories() -> Result<Vec<String>, String> {
    load_schema_array(OPERATION_SCHEMA_PATH, "/$defs/stable_error_category/enum")
}

fn load_schema_array(path: &str, pointer: &str) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(workspace_path(path))
        .map_err(|error| format!("failed to read `{path}`: {error}"))?;
    let root: Value = serde_json::from_str(&source)
        .map_err(|error| format!("failed to parse `{path}` JSON: {error}"))?;
    let value = root
        .pointer(pointer)
        .ok_or_else(|| format!("JSON pointer `{pointer}` not found in `{path}`"))?;
    extract_string_array(value, path, pointer)
}

fn extract_string_array(value: &Value, path: &str, pointer: &str) -> Result<Vec<String>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("`{path}` value at `{pointer}` is not an array"))?;
    let mut output = Vec::with_capacity(values.len());
    for entry in values {
        let Some(item) = entry.as_str() else {
            return Err(format!(
                "`{path}` value at `{pointer}` contains a non-string entry"
            ));
        };
        output.push(item.to_owned());
    }
    Ok(output)
}

fn validate_manifest_structure(manifest: &ConformanceManifest) -> Result<(), String> {
    if manifest.manifest_version != 1 {
        return Err(format!(
            "unsupported manifest_version {} (expected 1)",
            manifest.manifest_version
        ));
    }
    if manifest.operation_result_schema != "adapter-conformance-operation-result-v1.schema.json" {
        return Err(format!(
            "unexpected operation_result_schema `{}`",
            manifest.operation_result_schema
        ));
    }

    let stable_categories = manifest
        .stable_error_categories
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let reference_capabilities = manifest
        .reference_driver
        .capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if reference_capabilities.len() != manifest.reference_driver.capabilities.len() {
        return Err("reference_driver.capabilities contains duplicates".to_owned());
    }

    let mut seen_ids = BTreeSet::new();
    let mut scenario_titles = BTreeMap::new();
    for scenario in &manifest.scenarios {
        if !seen_ids.insert(scenario.id.clone()) {
            return Err(format!("duplicate scenario id `{}`", scenario.id));
        }
        scenario_titles.insert(scenario.id.clone(), scenario.title.clone());

        if matches!(
            scenario.execution_mode,
            ExecutionMode::AdapterRunnerRequired
        ) && scenario.adapter_runner_notes.is_none()
        {
            return Err(format!(
                "scenario `{}` is adapter_runner_required but missing adapter_runner_notes",
                scenario.id
            ));
        }

        if let Some(policy) = scenario.changed_asset_policy
            && !matches!(
                policy,
                ChangedAssetPolicy::RejectRefreshUntilSessionEnds
                    | ChangedAssetPolicy::ReloadForNextSessionOnly
                    | ChangedAssetPolicy::RestartRequired
            )
        {
            return Err(format!(
                "scenario `{}` has an unknown changed_asset_policy",
                scenario.id
            ));
        }

        if let Some(expected_error) = &scenario.expected_error {
            for category in expected_error.categories() {
                if !stable_categories.contains(&category) {
                    return Err(format!(
                        "scenario `{}` expected_error references unknown stable category `{category}`",
                        scenario.id
                    ));
                }
            }
        }

        let mut error_step_count = 0usize;
        for step in &scenario.steps {
            match step.expect.status {
                StepStatus::Ok => {
                    if step.expect.error_category.is_some()
                        || step.expect.allowed_error_categories.is_some()
                    {
                        return Err(format!(
                            "scenario `{}` has ok step with error fields",
                            scenario.id
                        ));
                    }
                }
                StepStatus::Error => {
                    error_step_count += 1;
                    let Some(step_error) = step.expect.expected_error() else {
                        return Err(format!(
                            "scenario `{}` has error step missing category mapping",
                            scenario.id
                        ));
                    };

                    if let Some(expected_error) = &scenario.expected_error {
                        if step_error != *expected_error {
                            return Err(format!(
                                "scenario `{}` step error mapping differs from scenario expected_error",
                                scenario.id
                            ));
                        }
                    } else {
                        return Err(format!(
                            "scenario `{}` has an error step but no scenario expected_error",
                            scenario.id
                        ));
                    }
                    for category in step_error.categories() {
                        if !stable_categories.contains(&category) {
                            return Err(format!(
                                "scenario `{}` step references unknown stable category `{category}`",
                                scenario.id
                            ));
                        }
                    }
                }
            }
        }

        let expected_error_steps = usize::from(scenario.expected_error.is_some());
        if error_step_count != expected_error_steps {
            return Err(format!(
                "scenario `{}` must include exactly {expected_error_steps} error step(s) (found {error_step_count})",
                scenario.id,
            ));
        }
    }

    if scenario_titles.is_empty() {
        return Err("scenario manifest contains no scenarios".to_owned());
    }

    Ok(())
}

pub(crate) fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}
