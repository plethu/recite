#![cfg(test)]

#[path = "adapter_conformance/availability.rs"]
mod availability;
#[path = "adapter_conformance/driver.rs"]
mod driver;
#[path = "adapter_conformance/manifest.rs"]
mod manifest;

use std::collections::BTreeSet;
use std::fs;

use driver::{ReferenceDriver, StepResult};
use manifest::{
    AvailabilityReasonTreeExpectation, Capability, ExecutionMode, PluralAttemptOutcomeExpectation,
    PluralResolutionOutcomeExpectation, RequirementLevel, StepStatus,
    load_contract_error_categories, load_manifest, load_manifest_schema_error_categories,
    load_operation_schema_error_categories,
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
fn availability_reason_fields_stay_in_sync_across_schema_manifest_and_reference_driver() {
    let schema_source = fs::read_to_string(manifest::workspace_path(
        "fixtures/adapter-conformance/v1/adapter-conformance-operation-result-v1.schema.json",
    ))
    .expect("operation/result schema reads");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_source).expect("operation/result schema parses");
    for pointer in [
        "/$defs/operation/oneOf/0/properties/schema_fixture",
        "/$defs/step_expectation/properties/prompt_choice_availability",
        "/$defs/choice_availability",
        "/$defs/availability_reason",
        "/$defs/availability_reason_origin",
        "/$defs/availability_reason_tree",
        "/$defs/availability_reason_arg",
        "/$defs/availability_reason_value",
    ] {
        assert!(
            schema.pointer(pointer).is_some(),
            "operation/result schema is missing `{pointer}`"
        );
    }

    let manifest = load_manifest().expect("conformance manifest loads");
    let scenario = manifest
        .scenarios
        .iter()
        .find(|scenario| scenario.id == "unavailable_choice_error_conditioned_choice")
        .expect("availability reason scenario is present");
    let availability = scenario
        .steps
        .iter()
        .find_map(|step| step.expect.prompt_choice_availability.as_ref())
        .expect("availability reason scenario asserts prompt_choice_availability");

    assert!(
        availability
            .iter()
            .any(|choice| choice.is_available && choice.primary_reason.is_none()),
        "availability scenario must keep at least one available choice in the structured output"
    );
    assert!(
        availability
            .iter()
            .any(|choice| !choice.is_available && choice.primary_reason.is_some()),
        "availability scenario must assert an explicit primary reason"
    );
    assert!(
        availability.iter().any(|choice| matches!(
            choice.reason_tree.as_ref(),
            Some(AvailabilityReasonTreeExpectation::All { .. })
        )),
        "availability scenario must assert an all-group reason tree"
    );
    assert!(
        availability.iter().any(|choice| matches!(
            choice.reason_tree.as_ref(),
            Some(AvailabilityReasonTreeExpectation::Any { .. })
        )),
        "availability scenario must assert an any-group reason tree"
    );
}

#[test]
fn plural_adapter_scenario_declares_structured_line_metadata() {
    let manifest = load_manifest().expect("conformance manifest loads");
    let scenario = manifest
        .scenarios
        .iter()
        .find(|scenario| scenario.id == "plural_line_structured_metadata_adapter_runner_required")
        .expect("plural adapter scenario is present");
    let plural = scenario
        .steps
        .iter()
        .find_map(|step| step.expect.line_plural.as_ref())
        .expect("plural adapter scenario asserts line_plural");

    assert_eq!(plural.singular_source_text, "You have one letter.");
    assert_eq!(plural.plural_source_text, "You have {count} letters.");
    assert_eq!(plural.count, 2);
    assert_eq!(plural.selected_arm, 1);
    assert_eq!(plural.resolution.matched_locale.as_deref(), Some("fr-FR"));
    assert_eq!(
        plural.resolution.matched_context.as_deref(),
        Some("5fcf9a1f7b20211f4a92")
    );
    assert_eq!(
        plural.resolution.matched_key.as_deref(),
        Some("5fcf9a1f7b20211f4a92")
    );
    assert_eq!(plural.resolution.matched_arm, Some(1));
    assert_eq!(plural.resolution.source_fallback_arm, None);
    assert_eq!(
        plural.resolution.outcome,
        PluralResolutionOutcomeExpectation::Translated
    );
    assert_eq!(plural.resolution.attempts.len(), 1);
    let attempt = &plural.resolution.attempts[0];
    assert_eq!(attempt.locale, "fr-FR");
    assert_eq!(attempt.context, "5fcf9a1f7b20211f4a92");
    assert_eq!(attempt.key, "5fcf9a1f7b20211f4a92");
    assert_eq!(attempt.selected_arm, Some(1));
    assert_eq!(attempt.outcome, PluralAttemptOutcomeExpectation::Matched);
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
                    if let Some(expected_availability) = &step.expect.prompt_choice_availability {
                        assert_eq!(
                            outcome.prompt_choice_availability.as_ref(),
                            Some(expected_availability),
                            "scenario `{}` step {} expected prompt choice availability {:?}",
                            scenario.id,
                            step_index + 1,
                            expected_availability
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
