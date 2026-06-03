#![cfg(test)]

#[path = "adapter_conformance/driver.rs"]
mod driver;
#[path = "adapter_conformance/manifest.rs"]
mod manifest;

use std::collections::BTreeSet;

use driver::{ReferenceDriver, StepResult};
use manifest::{
    Capability, ExecutionMode, RequirementLevel, StepStatus, load_contract_error_categories,
    load_manifest, load_manifest_schema_error_categories, load_operation_schema_error_categories,
};

const PROJECTION_ERROR_CATEGORIES: [&str; 3] = [
    "missing_projection_handler_error",
    "projection_evaluation_error",
    "invalid_projection_result_error",
];

#[test]
fn stable_error_category_table_stays_in_sync_with_contract_and_schema_artifacts() {
    let manifest = load_manifest().expect("conformance manifest loads");
    let contract_categories =
        load_contract_error_categories().expect("contract categories parse from docs");
    let manifest_schema_categories =
        load_manifest_schema_error_categories().expect("manifest schema categories parse");
    let operation_schema_categories =
        load_operation_schema_error_categories().expect("operation schema categories parse");

    assert_eq!(
        manifest.stable_error_categories, contract_categories,
        "scenario manifest stable_error_categories drifted from docs/engine-adapter-contract.md §12"
    );
    assert_eq!(
        manifest_schema_categories, contract_categories,
        "manifest schema stable_error_categories drifted from docs/engine-adapter-contract.md §12"
    );
    assert_eq!(
        operation_schema_categories, contract_categories,
        "operation/result schema stable_error_category enum drifted from docs/engine-adapter-contract.md §12"
    );
}

#[test]
fn reference_driver_runs_reference_scenarios_and_checks_mandatory_category_coverage() {
    let manifest = load_manifest().expect("conformance manifest loads");
    let contract_categories =
        load_contract_error_categories().expect("contract categories parse from docs");

    let mandatory_categories = manifest
        .scenarios
        .iter()
        .filter(|scenario| matches!(scenario.requirement_level, RequirementLevel::Mandatory))
        .filter_map(|scenario| scenario.expected_error.as_ref())
        .flat_map(|expected_error| expected_error.categories())
        .collect::<BTreeSet<_>>();
    let contract_categories_set = contract_categories.into_iter().collect::<BTreeSet<_>>();
    let projection_categories = PROJECTION_ERROR_CATEGORIES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mandatory_contract_categories = contract_categories_set
        .difference(&projection_categories)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        mandatory_categories, mandatory_contract_categories,
        "mandatory scenario set must cover every non-projection stable category from docs/engine-adapter-contract.md §12"
    );

    let projection_capability_categories = manifest
        .scenarios
        .iter()
        .filter(|scenario| {
            matches!(
                scenario.requirement_level,
                RequirementLevel::CapabilityGated
            ) && scenario
                .capability_gates
                .contains(&Capability::PresentationProjection)
        })
        .filter_map(|scenario| scenario.expected_error.as_ref())
        .flat_map(|expected_error| expected_error.categories())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        projection_capability_categories, projection_categories,
        "presentation_projection scenarios must cover every projection stable error category from docs/engine-adapter-contract.md §12"
    );

    let mut executed = 0usize;
    let mut skipped_adapter_runner_required = 0usize;
    let mut skipped_capability_gated = 0usize;
    for scenario in &manifest.scenarios {
        if matches!(
            scenario.execution_mode,
            ExecutionMode::AdapterRunnerRequired
        ) {
            skipped_adapter_runner_required += 1;
            continue;
        }
        if !manifest
            .reference_driver
            .capabilities
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .is_superset(&scenario.capability_gates.iter().copied().collect())
        {
            skipped_capability_gated += 1;
            continue;
        }
        let driver_config = manifest.reference_driver.with_policy_override(
            scenario
                .changed_asset_policy
                .unwrap_or(manifest.reference_driver.changed_asset_policy),
        );
        let mut driver = ReferenceDriver::new(&driver_config);
        assert!(
            driver.supports_capabilities(&scenario.capability_gates),
            "scenario `{}` should have been skipped due to unsupported capability gates",
            scenario.id
        );

        for (step_index, step) in scenario.steps.iter().enumerate() {
            let result = driver.execute(&step.operation);
            match (&step.expect.status, result) {
                (StepStatus::Ok, StepResult::Ok(outcome)) => {
                    if let Some(expected_event_kind) = step.expect.event_kind {
                        assert_eq!(
                            outcome.event_kind,
                            Some(expected_event_kind),
                            "scenario `{}` step {} expected event kind {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_event_kind
                        );
                    }
                    if let Some(expected_line_text) = &step.expect.line_text {
                        assert_eq!(
                            outcome.line_text.as_ref(),
                            Some(expected_line_text),
                            "scenario `{}` step {} expected line text `{}`",
                            scenario.id,
                            step_index + 1,
                            expected_line_text
                        );
                    }
                    if let Some(expected_choice_ids) = &step.expect.prompt_choice_ids {
                        assert_eq!(
                            outcome.prompt_choice_ids.as_ref(),
                            Some(expected_choice_ids),
                            "scenario `{}` step {} expected prompt choice IDs {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_choice_ids
                        );
                    }
                    if let Some(expected_unavailable_ids) =
                        &step.expect.prompt_unavailable_choice_ids
                    {
                        assert_eq!(
                            outcome.prompt_unavailable_choice_ids.as_ref(),
                            Some(expected_unavailable_ids),
                            "scenario `{}` step {} expected unavailable choice IDs {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_unavailable_ids
                        );
                    }
                    if let Some(expected_effect_function) = &step.expect.effect_function {
                        assert_eq!(
                            outcome.effect_function.as_ref(),
                            Some(expected_effect_function),
                            "scenario `{}` step {} expected effect function `{}`",
                            scenario.id,
                            step_index + 1,
                            expected_effect_function
                        );
                    }
                    if let Some(expected_effect_mode) = step.expect.effect_mode {
                        assert_eq!(
                            outcome.effect_mode,
                            Some(expected_effect_mode),
                            "scenario `{}` step {} expected effect mode {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_effect_mode
                        );
                    }
                    if let Some(expected_deferred_functions) =
                        &step.expect.deferred_effect_functions
                    {
                        assert_eq!(
                            outcome.deferred_effect_functions.as_ref(),
                            Some(expected_deferred_functions),
                            "scenario `{}` step {} expected deferred effect functions {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_deferred_functions
                        );
                    }
                    if let Some(effect_slot) = &step.expect.pending_effect_slot {
                        let remembered =
                            driver.remembered_effect_id(effect_slot).unwrap_or_else(|| {
                                panic!(
                                    "scenario `{}` step {} expected remembered effect slot `{}`",
                                    scenario.id,
                                    step_index + 1,
                                    effect_slot
                                )
                            });
                        assert_eq!(
                            outcome.pending_effect_id.as_deref(),
                            Some(remembered),
                            "scenario `{}` step {} expected pending effect ID from slot `{}`",
                            scenario.id,
                            step_index + 1,
                            effect_slot
                        );
                    }
                }
                (StepStatus::Error, StepResult::Error { category, detail }) => {
                    if let Some(expected_category) = &step.expect.error_category {
                        assert_eq!(
                            &category,
                            expected_category,
                            "scenario `{}` step {} expected error category `{}` but got `{}` ({detail})",
                            scenario.id,
                            step_index + 1,
                            expected_category,
                            category
                        );
                    } else if let Some(allowed_categories) = &step.expect.allowed_error_categories {
                        assert!(
                            allowed_categories.contains(&category),
                            "scenario `{}` step {} expected one of {:?}, got `{}` ({detail})",
                            scenario.id,
                            step_index + 1,
                            allowed_categories,
                            category
                        );
                    } else {
                        panic!(
                            "scenario `{}` step {} has StepStatus::Error but no expected categories",
                            scenario.id,
                            step_index + 1
                        );
                    }
                }
                (StepStatus::Ok, StepResult::Error { category, detail }) => {
                    panic!(
                        "scenario `{}` step {} expected success but got `{}` ({detail})",
                        scenario.id,
                        step_index + 1,
                        category
                    );
                }
                (StepStatus::Error, StepResult::Ok(outcome)) => {
                    panic!(
                        "scenario `{}` step {} expected error but got success outcome {:?}",
                        scenario.id,
                        step_index + 1,
                        outcome
                    );
                }
            }
        }

        executed += 1;
    }

    assert!(
        executed > 0,
        "expected to execute at least one reference scenario"
    );
    assert!(
        skipped_adapter_runner_required > 0,
        "expected at least one adapter_runner_required scenario to remain documented but skipped by the reference driver"
    );
    assert_policy_scenarios_cover_all_declared_policies(&manifest);

    let _ = skipped_capability_gated;
}

fn assert_policy_scenarios_cover_all_declared_policies(manifest: &manifest::ConformanceManifest) {
    let policies = manifest
        .scenarios
        .iter()
        .filter_map(|scenario| scenario.changed_asset_policy)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        policies,
        [
            manifest::ChangedAssetPolicy::RejectRefreshUntilSessionEnds,
            manifest::ChangedAssetPolicy::ReloadForNextSessionOnly,
            manifest::ChangedAssetPolicy::RestartRequired,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        "changed-asset conformance scenarios must cover every declared policy"
    );
}
