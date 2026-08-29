use std::collections::BTreeMap;

use serde::Deserialize;

use super::expectations::StepExpectation;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConformanceManifest {
    pub(crate) manifest_version: u32,
    pub(crate) operation_result_schema: String,
    pub(crate) stable_error_categories: Vec<String>,
    pub(crate) reference_driver: ReferenceDriverConfig,
    pub(crate) scenarios: Vec<Scenario>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferenceDriverConfig {
    pub(crate) changed_asset_policy: ChangedAssetPolicy,
    pub(crate) capabilities: Vec<Capability>,
}

impl ReferenceDriverConfig {
    pub(crate) fn with_policy_override(&self, changed_asset_policy: ChangedAssetPolicy) -> Self {
        Self {
            changed_asset_policy,
            capabilities: self.capabilities.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChangedAssetPolicy {
    RejectRefreshUntilSessionEnds,
    ReloadForNextSessionOnly,
    RestartRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Capability {
    SourceImportVisibility,
    SchemaImportVisibility,
    PresentationProjection,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RequirementLevel {
    Mandatory,
    CapabilityGated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionMode {
    ReferenceDriver,
    AdapterRunnerRequired,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scenario {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) requirement_level: RequirementLevel,
    pub(crate) execution_mode: ExecutionMode,
    pub(crate) capability_gates: Vec<Capability>,
    pub(crate) changed_asset_policy: Option<ChangedAssetPolicy>,
    pub(crate) expected_error: Option<ExpectedError>,
    pub(crate) adapter_runner_notes: Option<String>,
    pub(crate) steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScenarioStep {
    pub(crate) operation: Operation,
    pub(crate) expect: StepExpectation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub(crate) enum ExpectedError {
    Single {
        error_category: String,
    },
    Allowed {
        allowed_error_categories: Vec<String>,
    },
}

impl ExpectedError {
    pub(crate) fn categories(&self) -> Vec<String> {
        match self {
            Self::Single { error_category } => vec![error_category.clone()],
            Self::Allowed {
                allowed_error_categories,
            } => allowed_error_categories.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Operation {
    CompileFixture {
        fixture: String,
        asset_slot: String,
        asset_id: Option<String>,
        schema_fixture: Option<String>,
        schema_fingerprint: Option<String>,
    },
    CompileInvalidFixture {
        fixture: String,
    },
    DecodeCompiledBytes {
        bytes_case: BytesCase,
    },
    ImportAsset {
        asset_slot: String,
        declared_source_fingerprint: Option<String>,
        declared_schema_fingerprint: Option<String>,
    },
    StartSession {
        asset_slot: String,
        block: Option<String>,
        locale: Option<String>,
        interpolation_values: Option<BTreeMap<String, i64>>,
    },
    ExerciseLocalisationFailure {
        asset_slot: String,
        locale: String,
    },
    ExerciseProjectionFailure {
        asset_slot: String,
        projection_id: String,
        failure_kind: ProjectionFailureKind,
    },
    Advance {
        asset_slot: Option<String>,
    },
    Choose {
        choice_id: String,
    },
    ChooseFromSlot {
        slot: String,
    },
    RememberPromptChoice {
        index: usize,
        slot: String,
    },
    RememberPendingEffectId {
        slot: String,
    },
    AcknowledgeEffect {
        ack: AckKind,
        effect_id: Option<String>,
        effect_slot: Option<String>,
        failure_reason: Option<String>,
    },
    SaveSession {
        snapshot_slot: String,
    },
    LoadSession {
        snapshot_slot: String,
        asset_slot: String,
    },
    MutateSnapshotFormat {
        input_snapshot_slot: String,
        output_snapshot_slot: String,
        snapshot_format_version: u16,
    },
    EndSession,
    ClearConditionHandlers,
    SetConditionBehavior {
        function: String,
        behavior_kind: ConditionBehaviorKind,
        bool_value: Option<bool>,
        enum_value: Option<String>,
        failure_reason: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BytesCase {
    TruncatedMessagepack,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AckKind {
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConditionBehaviorKind {
    Bool,
    Enum,
    Failure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProjectionFailureKind {
    MissingHandler,
    EvaluationFailure,
    InvalidResult,
}
