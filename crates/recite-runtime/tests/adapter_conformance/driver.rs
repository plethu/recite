use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, ContentFingerprint, EffectId,
    LocaleId, SchemaFingerprint, SourceMapId, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack,
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContentFingerprintSnapshot,
    DialogueContext, DialogueError, DialogueEvent, DialogueSchemaFingerprintSnapshot,
    DialogueSession, DialogueSessionOptions, EffectAck, acknowledge_effect, choose, next,
    restore_session, snapshot_session, start_scene, start_scene_with_options,
};

use super::availability::choice_availability_expectation;
use super::manifest::workspace_path;
use super::manifest::{
    AckKind, BytesCase, Capability, ChangedAssetPolicy, ChoiceAvailabilityExpectation,
    ConditionBehaviorKind, EffectMode, EventKind, Operation, ProjectionFailureKind,
    ReferenceDriverConfig,
};

const CATEGORY_VALIDATION_ERROR: &str = "validation_error";
const CATEGORY_ASSET_LOAD_OR_DECODE_ERROR: &str = "asset_load_or_decode_error";
const CATEGORY_STALE_OR_INCOMPATIBLE_ASSET_ERROR: &str = "stale_or_incompatible_asset_error";
const CATEGORY_SCHEMA_MISMATCH_ERROR: &str = "schema_mismatch_error";
const CATEGORY_NO_ACTIVE_SESSION_ERROR: &str = "no_active_session_error";
const CATEGORY_SESSION_ALREADY_ACTIVE_ERROR: &str = "session_already_active_error";
const CATEGORY_UNKNOWN_START_BLOCK_ERROR: &str = "unknown_start_block_error";
const CATEGORY_INVALID_CHOICE_ERROR: &str = "invalid_choice_error";
const CATEGORY_UNAVAILABLE_CHOICE_ERROR: &str = "unavailable_choice_error";
const CATEGORY_STALE_CHOICE_ERROR: &str = "stale_choice_error";
const CATEGORY_MISSING_CONDITION_HANDLER_ERROR: &str = "missing_condition_handler_error";
const CATEGORY_CONDITION_EVALUATION_ERROR: &str = "condition_evaluation_error";
const CATEGORY_INVALID_CONDITION_RESULT_ERROR: &str = "invalid_condition_result_error";
const CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR: &str = "effect_acknowledgement_error";
const CATEGORY_REJECTED_CHANGED_ASSET_REFRESH_ERROR: &str = "rejected_changed_asset_refresh_error";
const CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR: &str = "save_load_incompatibility_error";
const CATEGORY_LOCALISATION_ERROR: &str = "localisation_error";
const CATEGORY_MISSING_PROJECTION_HANDLER_ERROR: &str = "missing_projection_handler_error";
const CATEGORY_PROJECTION_EVALUATION_ERROR: &str = "projection_evaluation_error";
const CATEGORY_INVALID_PROJECTION_RESULT_ERROR: &str = "invalid_projection_result_error";

#[derive(Clone, Debug)]
struct CompiledSlot {
    dialogue: CompiledDialogue,
    source_fingerprint_label: String,
    schema_fingerprint_label: String,
}

#[derive(Clone, Debug)]
struct ImportedSlot {
    dialogue: CompiledDialogue,
}

#[derive(Clone, Debug)]
struct ActiveSession {
    asset_slot: String,
    session: DialogueSession,
}

#[derive(Clone, Debug)]
enum ConditionBehavior {
    Bool(bool),
    Enum(String),
    Failure(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StepOutcome {
    pub(crate) event_kind: Option<EventKind>,
    pub(crate) line_text: Option<String>,
    pub(crate) prompt_choice_ids: Option<Vec<String>>,
    pub(crate) prompt_unavailable_choice_ids: Option<Vec<String>>,
    pub(crate) prompt_choice_availability: Option<Vec<ChoiceAvailabilityExpectation>>,
    pub(crate) effect_function: Option<String>,
    pub(crate) effect_mode: Option<EffectMode>,
    pub(crate) deferred_effect_functions: Option<Vec<String>>,
    pub(crate) pending_effect_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StepResult {
    Ok(StepOutcome),
    Error { category: String, detail: String },
}

pub(crate) struct ReferenceDriver {
    changed_asset_policy: ChangedAssetPolicy,
    capabilities: BTreeSet<Capability>,
    compiled_assets: BTreeMap<String, CompiledSlot>,
    imported_assets: BTreeMap<String, ImportedSlot>,
    active_session: Option<ActiveSession>,
    snapshots: BTreeMap<String, recite_runtime::DialogueSessionSnapshot>,
    remembered_choice_ids: BTreeMap<String, String>,
    remembered_effect_ids: BTreeMap<String, String>,
    condition_behaviors: BTreeMap<String, ConditionBehavior>,
    seen_choice_ids: BTreeSet<String>,
    current_prompt_choice_ids: BTreeSet<String>,
    last_event: Option<DialogueEvent>,
}

impl ReferenceDriver {
    pub(crate) fn new(config: &ReferenceDriverConfig) -> Self {
        Self {
            changed_asset_policy: config.changed_asset_policy,
            capabilities: config.capabilities.iter().copied().collect(),
            compiled_assets: BTreeMap::new(),
            imported_assets: BTreeMap::new(),
            active_session: None,
            snapshots: BTreeMap::new(),
            remembered_choice_ids: BTreeMap::new(),
            remembered_effect_ids: BTreeMap::new(),
            condition_behaviors: BTreeMap::new(),
            seen_choice_ids: BTreeSet::new(),
            current_prompt_choice_ids: BTreeSet::new(),
            last_event: None,
        }
    }

    pub(crate) fn supports_capabilities(&self, gates: &[Capability]) -> bool {
        gates.iter().all(|gate| self.capabilities.contains(gate))
    }

    pub(crate) fn remembered_effect_id(&self, slot: &str) -> Option<&str> {
        self.remembered_effect_ids.get(slot).map(String::as_str)
    }

    pub(crate) fn execute(&mut self, operation: &Operation) -> StepResult {
        match operation {
            Operation::CompileFixture {
                fixture,
                asset_slot,
                asset_id,
                schema_fixture,
                schema_fingerprint,
            } => self.compile_fixture(
                fixture,
                asset_slot,
                asset_id.as_deref(),
                schema_fixture.as_deref(),
                schema_fingerprint,
            ),
            Operation::CompileInvalidFixture { fixture } => self.compile_invalid_fixture(fixture),
            Operation::DecodeCompiledBytes { bytes_case } => {
                self.decode_compiled_bytes(*bytes_case)
            }
            Operation::ImportAsset {
                asset_slot,
                declared_source_fingerprint,
                declared_schema_fingerprint,
            } => self.import_asset(
                asset_slot,
                declared_source_fingerprint.as_deref(),
                declared_schema_fingerprint.as_deref(),
            ),
            Operation::StartSession {
                asset_slot,
                block,
                locale,
            } => self.start_session(asset_slot, block.as_deref(), locale.as_deref()),
            Operation::ExerciseLocalisationFailure { asset_slot, locale } => {
                self.exercise_localisation_failure(asset_slot, locale)
            }
            Operation::ExerciseProjectionFailure {
                asset_slot,
                projection_id,
                failure_kind,
            } => self.exercise_projection_failure(asset_slot, projection_id, *failure_kind),
            Operation::Advance { asset_slot } => self.advance(asset_slot.as_deref()),
            Operation::Choose { choice_id } => self.choose(choice_id),
            Operation::ChooseFromSlot { slot } => self.choose_from_slot(slot),
            Operation::RememberPromptChoice { index, slot } => {
                self.remember_prompt_choice(*index, slot)
            }
            Operation::RememberPendingEffectId { slot } => self.remember_pending_effect_id(slot),
            Operation::AcknowledgeEffect {
                ack,
                effect_id,
                effect_slot,
                failure_reason,
            } => self.acknowledge_effect(
                *ack,
                effect_id.as_deref(),
                effect_slot.as_deref(),
                failure_reason.as_deref(),
            ),
            Operation::SaveSession { snapshot_slot } => self.save_session(snapshot_slot),
            Operation::LoadSession {
                snapshot_slot,
                asset_slot,
            } => self.load_session(snapshot_slot, asset_slot),
            Operation::MutateSnapshotFormat {
                input_snapshot_slot,
                output_snapshot_slot,
                snapshot_format_version,
            } => self.mutate_snapshot_format(
                input_snapshot_slot,
                output_snapshot_slot,
                *snapshot_format_version,
            ),
            Operation::EndSession => {
                self.active_session = None;
                self.last_event = None;
                self.current_prompt_choice_ids.clear();
                StepResult::Ok(self.empty_outcome())
            }
            Operation::ClearConditionHandlers => {
                self.condition_behaviors.clear();
                StepResult::Ok(self.empty_outcome())
            }
            Operation::SetConditionBehavior {
                function,
                behavior_kind,
                bool_value,
                enum_value,
                failure_reason,
            } => self.set_condition_behavior(
                function,
                *behavior_kind,
                *bool_value,
                enum_value.as_deref(),
                failure_reason.as_deref(),
            ),
        }
    }

    fn compile_fixture(
        &mut self,
        fixture: &str,
        asset_slot: &str,
        asset_id: Option<&str>,
        schema_fixture: Option<&str>,
        schema_fingerprint_token: &Option<String>,
    ) -> StepResult {
        let source = match fs::read_to_string(workspace_path(fixture)) {
            Ok(source) => source,
            Err(error) => {
                return self.error(
                    CATEGORY_VALIDATION_ERROR,
                    format!("failed to read fixture `{fixture}`: {error}"),
                );
            }
        };

        let asset_id = asset_id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("dialogue/{asset_slot}.recitec"));
        let asset_id = match CompiledAssetId::new(asset_id) {
            Ok(asset_id) => asset_id,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };
        let source_map_id = match SourceMapId::new(format!("dialogue/{asset_slot}.recitec.map")) {
            Ok(source_map_id) => source_map_id,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };
        let compiler_version = match CompilerVersion::new("0.0.1") {
            Ok(version) => version,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };
        let schema = match schema_fixture {
            Some(path) => {
                let schema_source = match fs::read_to_string(workspace_path(path)) {
                    Ok(source) => source,
                    Err(error) => {
                        return self.error(
                            CATEGORY_VALIDATION_ERROR,
                            format!("failed to read schema fixture `{path}`: {error}"),
                        );
                    }
                };
                match recite_core::load_schema_manifest_str(path, &schema_source).schema {
                    Some(schema) => Some(schema),
                    None => {
                        return self.error(
                            CATEGORY_VALIDATION_ERROR,
                            format!("schema fixture `{path}` did not load a schema"),
                        );
                    }
                }
            }
            None => None,
        };
        let (schema_fingerprint, schema_fingerprint_label) =
            schema_fingerprint_from_token(schema_fingerprint_token.as_deref(), schema.as_ref());

        let options = CompileOptions::new(
            compiler_version,
            asset_id,
            source_map_id,
            schema_fingerprint,
        );
        let input = [CompileInput::new(fixture, source)];
        let report = match if let Some(schema) = &schema {
            compile_inputs_with_schema(input, options, schema)
        } else {
            compile_inputs(input, options)
        } {
            Ok(report) => report,
            Err(error) => {
                return self.error(
                    CATEGORY_VALIDATION_ERROR,
                    format!("compile_inputs failed for `{fixture}`: {error}"),
                );
            }
        };

        if !report.diagnostics.is_empty() {
            return self.error(
                CATEGORY_VALIDATION_ERROR,
                format!(
                    "fixture `{fixture}` emitted {} diagnostics",
                    report.diagnostics.len()
                ),
            );
        }
        let Some(asset) = report.asset else {
            return self.error(
                CATEGORY_VALIDATION_ERROR,
                format!("fixture `{fixture}` compiled without diagnostics but emitted no asset"),
            );
        };

        let source_fingerprint_label = asset
            .dialogue
            .sources
            .iter()
            .map(|entry| format!("{}={}", entry.path, fingerprint_label(&entry.fingerprint)))
            .collect::<Vec<_>>()
            .join("|");

        self.compiled_assets.insert(
            asset_slot.to_owned(),
            CompiledSlot {
                dialogue: asset.dialogue,
                source_fingerprint_label,
                schema_fingerprint_label,
            },
        );

        StepResult::Ok(self.empty_outcome())
    }

    fn compile_invalid_fixture(&mut self, fixture: &str) -> StepResult {
        let source = match fs::read_to_string(workspace_path(fixture)) {
            Ok(source) => source,
            Err(error) => {
                return self.error(
                    CATEGORY_VALIDATION_ERROR,
                    format!("failed to read fixture `{fixture}`: {error}"),
                );
            }
        };

        let compiler_version = match CompilerVersion::new("0.0.1") {
            Ok(version) => version,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };
        let asset_id = match CompiledAssetId::new("dialogue/invalid.recitec") {
            Ok(asset_id) => asset_id,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };
        let source_map_id = match SourceMapId::new("dialogue/invalid.recitec.map") {
            Ok(source_map_id) => source_map_id,
            Err(error) => {
                return self.error(CATEGORY_VALIDATION_ERROR, error.to_string());
            }
        };

        let report = match compile_inputs(
            [CompileInput::new(fixture, source)],
            CompileOptions::new(
                compiler_version,
                asset_id,
                source_map_id,
                SchemaFingerprint::NoSchema,
            ),
        ) {
            Ok(report) => report,
            Err(error) => {
                return self.error(
                    CATEGORY_VALIDATION_ERROR,
                    format!("compile_inputs failed for invalid fixture `{fixture}`: {error}"),
                );
            }
        };

        if report.diagnostics.is_empty() {
            return self.error(
                CATEGORY_VALIDATION_ERROR,
                format!("expected diagnostics for `{fixture}` but compile was clean"),
            );
        }

        StepResult::Error {
            category: CATEGORY_VALIDATION_ERROR.to_owned(),
            detail: format!(
                "fixture `{fixture}` emitted {} diagnostics",
                report.diagnostics.len()
            ),
        }
    }

    fn decode_compiled_bytes(&mut self, bytes_case: BytesCase) -> StepResult {
        let bytes = match bytes_case {
            BytesCase::TruncatedMessagepack => vec![0x91, 0x92, 0x93],
        };

        match decode_compiled_dialogue_messagepack(&bytes) {
            Ok(_) => StepResult::Ok(self.empty_outcome()),
            Err(error) => self.error(CATEGORY_ASSET_LOAD_OR_DECODE_ERROR, error.to_string()),
        }
    }

    fn import_asset(
        &mut self,
        asset_slot: &str,
        declared_source_fingerprint: Option<&str>,
        declared_schema_fingerprint: Option<&str>,
    ) -> StepResult {
        let Some(compiled) = self.compiled_assets.get(asset_slot).cloned() else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("compiled asset slot `{asset_slot}` is not available"),
            );
        };

        if self.active_session.is_some() {
            match self.changed_asset_policy {
                ChangedAssetPolicy::RejectRefreshUntilSessionEnds
                | ChangedAssetPolicy::RestartRequired => {
                    return self.error(
                        CATEGORY_REJECTED_CHANGED_ASSET_REFRESH_ERROR,
                        format!(
                            "changed-asset policy `{}` rejected import for active session",
                            self.changed_asset_policy.as_str()
                        ),
                    );
                }
                ChangedAssetPolicy::ReloadForNextSessionOnly => {}
            }
        }

        if let Some(declared) = declared_source_fingerprint
            && self
                .capabilities
                .contains(&Capability::SourceImportVisibility)
            && declared != compiled.source_fingerprint_label
        {
            return self.error(
                CATEGORY_STALE_OR_INCOMPATIBLE_ASSET_ERROR,
                format!(
                    "declared source fingerprint `{declared}` != compiled `{}`",
                    compiled.source_fingerprint_label
                ),
            );
        }
        if let Some(declared) = declared_schema_fingerprint
            && self
                .capabilities
                .contains(&Capability::SchemaImportVisibility)
            && declared != compiled.schema_fingerprint_label
        {
            return self.error(
                CATEGORY_SCHEMA_MISMATCH_ERROR,
                format!(
                    "declared schema fingerprint `{declared}` != compiled `{}`",
                    compiled.schema_fingerprint_label
                ),
            );
        }

        self.imported_assets.insert(
            asset_slot.to_owned(),
            ImportedSlot {
                dialogue: compiled.dialogue,
            },
        );
        StepResult::Ok(self.empty_outcome())
    }

    fn start_session(
        &mut self,
        asset_slot: &str,
        block: Option<&str>,
        locale: Option<&str>,
    ) -> StepResult {
        if self.active_session.is_some() {
            return self.error(
                CATEGORY_SESSION_ALREADY_ACTIVE_ERROR,
                "session already active for declared owner".to_owned(),
            );
        }

        let Some(asset) = self.imported_assets.get(asset_slot) else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("imported asset slot `{asset_slot}` is not available"),
            );
        };

        let session_result = if let Some(locale) = locale {
            let locale = match LocaleId::new(locale.to_owned()) {
                Ok(locale) => locale,
                Err(error) => {
                    return self.error(CATEGORY_LOCALISATION_ERROR, error.to_string());
                }
            };
            start_scene_with_options(
                &asset.dialogue,
                block,
                DialogueSessionOptions::new().with_locale(locale),
            )
        } else {
            start_scene(&asset.dialogue, block)
        };

        let session = match session_result {
            Ok(session) => session,
            Err(error) => {
                let category = self.map_runtime_error(&error, None);
                return self.error(category, error.to_string());
            }
        };

        self.active_session = Some(ActiveSession {
            asset_slot: asset_slot.to_owned(),
            session,
        });
        self.last_event = None;
        self.current_prompt_choice_ids.clear();
        StepResult::Ok(self.outcome_without_event())
    }

    fn exercise_localisation_failure(&mut self, asset_slot: &str, locale: &str) -> StepResult {
        let _ = (asset_slot, locale);
        self.error(
            CATEGORY_LOCALISATION_ERROR,
            "adapter runner must accept localisation_error at import, start, or advance".to_owned(),
        )
    }

    fn exercise_projection_failure(
        &mut self,
        asset_slot: &str,
        projection_id: &str,
        failure_kind: ProjectionFailureKind,
    ) -> StepResult {
        let _ = (asset_slot, projection_id);
        let (category, detail) = match failure_kind {
            ProjectionFailureKind::MissingHandler => (
                CATEGORY_MISSING_PROJECTION_HANDLER_ERROR,
                "adapter runner must report missing projection query handlers",
            ),
            ProjectionFailureKind::EvaluationFailure => (
                CATEGORY_PROJECTION_EVALUATION_ERROR,
                "adapter runner must report projection handler evaluation failures",
            ),
            ProjectionFailureKind::InvalidResult => (
                CATEGORY_INVALID_PROJECTION_RESULT_ERROR,
                "adapter runner must report projection results outside the declared output contract",
            ),
        };
        self.error(category, detail.to_owned())
    }

    fn advance(&mut self, asset_slot_override: Option<&str>) -> StepResult {
        let Some(active_asset_slot) = self
            .active_session
            .as_ref()
            .map(|session| session.asset_slot.clone())
        else {
            return self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "advance requested with no active session".to_owned(),
            );
        };
        let selected_asset_slot = asset_slot_override.unwrap_or(active_asset_slot.as_str());
        let Some(asset) = self.imported_assets.get(selected_asset_slot).cloned() else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("imported asset slot `{selected_asset_slot}` is not available"),
            );
        };

        let context = DriverContext {
            behaviors: &self.condition_behaviors,
        };
        let runtime_result = {
            let Some(active_session) = self.active_session.as_mut() else {
                return self.error(
                    CATEGORY_NO_ACTIVE_SESSION_ERROR,
                    "active session disappeared before advance".to_owned(),
                );
            };
            next(&asset.dialogue, &mut active_session.session, &context)
        };

        match runtime_result {
            Ok(event) => {
                self.observe_event(&event);
                StepResult::Ok(self.outcome_from_event(&event))
            }
            Err(error) => {
                let category = self.map_runtime_error(&error, None);
                self.error(category, error.to_string())
            }
        }
    }

    fn choose(&mut self, choice_id: &str) -> StepResult {
        let parsed_choice_id = match ChoiceId::new(choice_id.to_owned()) {
            Ok(choice_id) => choice_id,
            Err(error) => {
                return self.error(CATEGORY_INVALID_CHOICE_ERROR, error.to_string());
            }
        };

        let Some(active_asset_slot) = self
            .active_session
            .as_ref()
            .map(|session| session.asset_slot.clone())
        else {
            return self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "choice selection requested with no active session".to_owned(),
            );
        };
        let Some(asset) = self.imported_assets.get(&active_asset_slot).cloned() else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("imported asset slot `{active_asset_slot}` is not available"),
            );
        };

        let context = DriverContext {
            behaviors: &self.condition_behaviors,
        };
        let runtime_result = {
            let Some(active_session) = self.active_session.as_mut() else {
                return self.error(
                    CATEGORY_NO_ACTIVE_SESSION_ERROR,
                    "active session disappeared before choose".to_owned(),
                );
            };
            choose(
                &asset.dialogue,
                &mut active_session.session,
                parsed_choice_id,
                &context,
            )
        };

        match runtime_result {
            Ok(event) => {
                self.observe_event(&event);
                StepResult::Ok(self.outcome_from_event(&event))
            }
            Err(error) => {
                let category = self.map_runtime_error(&error, Some(choice_id));
                self.error(category, error.to_string())
            }
        }
    }

    fn choose_from_slot(&mut self, slot: &str) -> StepResult {
        let Some(choice_id) = self.remembered_choice_ids.get(slot).cloned() else {
            return self.error(
                CATEGORY_INVALID_CHOICE_ERROR,
                format!("remembered choice slot `{slot}` is not available"),
            );
        };
        self.choose(&choice_id)
    }

    fn remember_prompt_choice(&mut self, index: usize, slot: &str) -> StepResult {
        let Some(DialogueEvent::Prompt { choices, .. }) = &self.last_event else {
            return self.error(
                CATEGORY_INVALID_CHOICE_ERROR,
                "remember_prompt_choice requires the previous event to be Prompt".to_owned(),
            );
        };
        let Some(choice) = choices.get(index) else {
            return self.error(
                CATEGORY_INVALID_CHOICE_ERROR,
                format!(
                    "remember_prompt_choice index {index} is out of range for {} choices",
                    choices.len()
                ),
            );
        };
        self.remembered_choice_ids
            .insert(slot.to_owned(), choice.id.as_str().to_owned());
        StepResult::Ok(self.outcome_without_event())
    }

    fn remember_pending_effect_id(&mut self, slot: &str) -> StepResult {
        let pending_effect_id = self
            .active_session
            .as_ref()
            .and_then(|active| active.session.pending_effect())
            .map(|effect| effect.id.as_str().to_owned());
        let Some(effect_id) = pending_effect_id else {
            return self.error(
                CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR,
                "remember_pending_effect_id requires a pending blocking effect".to_owned(),
            );
        };

        self.remembered_effect_ids
            .insert(slot.to_owned(), effect_id);
        StepResult::Ok(self.outcome_without_event())
    }

    fn acknowledge_effect(
        &mut self,
        ack_kind: AckKind,
        effect_id: Option<&str>,
        effect_slot: Option<&str>,
        failure_reason: Option<&str>,
    ) -> StepResult {
        let Some(active) = self.active_session.as_mut() else {
            return self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "acknowledge_effect requested with no active session".to_owned(),
            );
        };

        let resolved_effect_id = if let Some(slot) = effect_slot {
            let Some(remembered) = self.remembered_effect_ids.get(slot) else {
                return self.error(
                    CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR,
                    format!("remembered effect slot `{slot}` is not available"),
                );
            };
            remembered.clone()
        } else if let Some(effect_id) = effect_id {
            effect_id.to_owned()
        } else {
            return self.error(
                CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR,
                "acknowledge_effect requires effect_id or effect_slot".to_owned(),
            );
        };
        let effect_id = match EffectId::new(resolved_effect_id) {
            Ok(effect_id) => effect_id,
            Err(error) => {
                return self.error(CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR, error.to_string());
            }
        };
        let ack = match ack_kind {
            AckKind::Completed => EffectAck::Completed,
            AckKind::Failed => EffectAck::Failed {
                reason: failure_reason
                    .unwrap_or("acknowledged as failed")
                    .to_owned(),
            },
        };

        match acknowledge_effect(&mut active.session, effect_id, ack) {
            Ok(()) => StepResult::Ok(self.outcome_without_event()),
            Err(error) => {
                let category = self.map_runtime_error(&error, None);
                self.error(category, error.to_string())
            }
        }
    }

    fn save_session(&mut self, snapshot_slot: &str) -> StepResult {
        let Some(active) = self.active_session.as_ref() else {
            return self.error(
                CATEGORY_NO_ACTIVE_SESSION_ERROR,
                "save_session requested with no active session".to_owned(),
            );
        };
        self.snapshots
            .insert(snapshot_slot.to_owned(), snapshot_session(&active.session));
        StepResult::Ok(self.outcome_without_event())
    }

    fn load_session(&mut self, snapshot_slot: &str, asset_slot: &str) -> StepResult {
        if self.active_session.is_some() {
            return self.error(
                CATEGORY_SESSION_ALREADY_ACTIVE_ERROR,
                "load_session requested while a session is already active".to_owned(),
            );
        }

        let Some(snapshot) = self.snapshots.get(snapshot_slot).cloned() else {
            return self.error(
                CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR,
                format!("snapshot slot `{snapshot_slot}` is not available"),
            );
        };
        let Some(asset) = self.imported_assets.get(asset_slot).cloned() else {
            return self.error(
                CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
                format!("imported asset slot `{asset_slot}` is not available"),
            );
        };

        if snapshot.schema_fingerprint
            != schema_fingerprint_snapshot(&asset.dialogue.header.schema_fingerprint)
        {
            return self.error(
                CATEGORY_SCHEMA_MISMATCH_ERROR,
                format!(
                    "snapshot `{snapshot_slot}` schema fingerprint does not match imported asset `{asset_slot}`"
                ),
            );
        }

        match restore_session(&asset.dialogue, snapshot) {
            Ok(session) => {
                self.active_session = Some(ActiveSession {
                    asset_slot: asset_slot.to_owned(),
                    session,
                });
                self.last_event = None;
                self.current_prompt_choice_ids.clear();
                StepResult::Ok(self.outcome_without_event())
            }
            Err(error) => {
                let category = self.map_runtime_error(&error, None);
                self.error(category, error.to_string())
            }
        }
    }

    fn mutate_snapshot_format(
        &mut self,
        input_snapshot_slot: &str,
        output_snapshot_slot: &str,
        snapshot_format_version: u16,
    ) -> StepResult {
        let Some(snapshot) = self.snapshots.get(input_snapshot_slot).cloned() else {
            return self.error(
                CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR,
                format!("snapshot slot `{input_snapshot_slot}` is not available"),
            );
        };
        let mut mutated = snapshot;
        mutated.snapshot_format_version = snapshot_format_version;
        self.snapshots
            .insert(output_snapshot_slot.to_owned(), mutated);
        StepResult::Ok(self.outcome_without_event())
    }

    fn set_condition_behavior(
        &mut self,
        function: &str,
        behavior_kind: ConditionBehaviorKind,
        bool_value: Option<bool>,
        enum_value: Option<&str>,
        failure_reason: Option<&str>,
    ) -> StepResult {
        let behavior = match behavior_kind {
            ConditionBehaviorKind::Bool => {
                let Some(value) = bool_value else {
                    return self.error(
                        CATEGORY_VALIDATION_ERROR,
                        "set_condition_behavior bool requires bool_value".to_owned(),
                    );
                };
                ConditionBehavior::Bool(value)
            }
            ConditionBehaviorKind::Enum => {
                let Some(value) = enum_value else {
                    return self.error(
                        CATEGORY_VALIDATION_ERROR,
                        "set_condition_behavior enum requires enum_value".to_owned(),
                    );
                };
                ConditionBehavior::Enum(value.to_owned())
            }
            ConditionBehaviorKind::Failure => {
                let Some(reason) = failure_reason else {
                    return self.error(
                        CATEGORY_VALIDATION_ERROR,
                        "set_condition_behavior failure requires failure_reason".to_owned(),
                    );
                };
                ConditionBehavior::Failure(reason.to_owned())
            }
        };

        self.condition_behaviors
            .insert(function.to_owned(), behavior);
        StepResult::Ok(self.empty_outcome())
    }

    fn observe_event(&mut self, event: &DialogueEvent) {
        if let DialogueEvent::Prompt { choices, .. } = event {
            self.current_prompt_choice_ids = choices
                .iter()
                .map(|choice| choice.id.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            self.seen_choice_ids
                .extend(self.current_prompt_choice_ids.iter().cloned());
        } else {
            self.current_prompt_choice_ids.clear();
        }
        self.last_event = Some(event.clone());
    }

    fn outcome_from_event(&self, event: &DialogueEvent) -> StepOutcome {
        let mut outcome = StepOutcome {
            pending_effect_id: self.current_pending_effect_id(),
            ..StepOutcome::default()
        };

        match event {
            DialogueEvent::Line(_) => {
                outcome.event_kind = Some(EventKind::Line);
                if let DialogueEvent::Line(line) = event {
                    outcome.line_text = Some(line.text.clone());
                }
            }
            DialogueEvent::Prompt { choices, .. } => {
                outcome.event_kind = Some(EventKind::Prompt);
                outcome.prompt_choice_ids = Some(
                    choices
                        .iter()
                        .map(|choice| choice.id.as_str().to_owned())
                        .collect(),
                );
                outcome.prompt_unavailable_choice_ids = Some(
                    choices
                        .iter()
                        .filter(|choice| !choice.availability.is_available)
                        .map(|choice| choice.id.as_str().to_owned())
                        .collect(),
                );
                outcome.prompt_choice_availability = Some(
                    choices
                        .iter()
                        .map(choice_availability_expectation)
                        .collect(),
                );
            }
            DialogueEvent::Effect(effect) => {
                outcome.event_kind = Some(EventKind::Effect);
                outcome.effect_function = Some(effect.function.clone());
                outcome.effect_mode = Some(match effect.mode {
                    recite_runtime::DialogueEffectMode::Deferred => EffectMode::Deferred,
                    recite_runtime::DialogueEffectMode::Immediate => EffectMode::Immediate,
                    recite_runtime::DialogueEffectMode::Blocking => EffectMode::Blocking,
                });
            }
            DialogueEvent::End { deferred_effects } => {
                outcome.event_kind = Some(EventKind::End);
                outcome.deferred_effect_functions = Some(
                    deferred_effects
                        .iter()
                        .map(|effect| effect.function.clone())
                        .collect(),
                );
            }
        }

        outcome
    }

    fn outcome_without_event(&self) -> StepOutcome {
        StepOutcome {
            pending_effect_id: self.current_pending_effect_id(),
            ..StepOutcome::default()
        }
    }

    fn empty_outcome(&self) -> StepOutcome {
        StepOutcome::default()
    }

    fn current_pending_effect_id(&self) -> Option<String> {
        self.active_session
            .as_ref()
            .and_then(|active| active.session.pending_effect())
            .map(|effect| effect.id.as_str().to_owned())
    }

    fn map_runtime_error(
        &self,
        error: &DialogueError,
        attempted_choice: Option<&str>,
    ) -> &'static str {
        match error {
            DialogueError::UnknownBlock { .. } => CATEGORY_UNKNOWN_START_BLOCK_ERROR,
            DialogueError::UnsupportedCompiledFormat { .. }
            | DialogueError::AssetMismatch { .. }
            | DialogueError::AssetContentMismatch { .. } => {
                CATEGORY_STALE_OR_INCOMPATIBLE_ASSET_ERROR
            }
            DialogueError::MalformedCompiledAsset { .. } => CATEGORY_ASSET_LOAD_OR_DECODE_ERROR,
            DialogueError::EffectPending { .. }
            | DialogueError::NoEffectPending { .. }
            | DialogueError::WrongEffectAcknowledgement { .. } => {
                CATEGORY_EFFECT_ACKNOWLEDGEMENT_ERROR
            }
            DialogueError::PromptPending { .. } => CATEGORY_INVALID_CHOICE_ERROR,
            DialogueError::NoPromptPending { .. } | DialogueError::InvalidChoice { .. } => {
                if let Some(choice_id) = attempted_choice
                    && self.seen_choice_ids.contains(choice_id)
                    && !self.current_prompt_choice_ids.contains(choice_id)
                {
                    return CATEGORY_STALE_CHOICE_ERROR;
                }
                CATEGORY_INVALID_CHOICE_ERROR
            }
            DialogueError::UnavailableChoice { .. } => CATEGORY_UNAVAILABLE_CHOICE_ERROR,
            DialogueError::ConditionEvaluationFailed { reason, .. } => {
                if reason.starts_with("no condition handler registered for `") {
                    CATEGORY_MISSING_CONDITION_HANDLER_ERROR
                } else {
                    CATEGORY_CONDITION_EVALUATION_ERROR
                }
            }
            DialogueError::ConditionResultTypeMismatch { .. } => {
                CATEGORY_INVALID_CONDITION_RESULT_ERROR
            }
            DialogueError::ConditionDepthLimitExceeded { .. } => {
                CATEGORY_CONDITION_EVALUATION_ERROR
            }
            DialogueError::UnsupportedSessionSnapshotFormat { .. }
            | DialogueError::SessionSnapshotEncodeFailed { .. }
            | DialogueError::SessionSnapshotDecodeFailed { .. }
            | DialogueError::InvalidSessionSnapshot { .. } => {
                CATEGORY_SAVE_LOAD_INCOMPATIBILITY_ERROR
            }
            DialogueError::SessionEnded => CATEGORY_NO_ACTIVE_SESSION_ERROR,
            DialogueError::TraversalLimitExceeded { .. } => {
                CATEGORY_STALE_OR_INCOMPATIBLE_ASSET_ERROR
            }
        }
    }

    fn error(&self, category: &str, detail: String) -> StepResult {
        StepResult::Error {
            category: category.to_owned(),
            detail,
        }
    }
}

impl ChangedAssetPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::RejectRefreshUntilSessionEnds => "reject_refresh_until_session_ends",
            Self::ReloadForNextSessionOnly => "reload_for_next_session_only",
            Self::RestartRequired => "restart_required",
        }
    }
}

struct DriverContext<'a> {
    behaviors: &'a BTreeMap<String, ConditionBehavior>,
}

impl DialogueContext for DriverContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let Some(behavior) = self.behaviors.get(query.function()) else {
            return Err(ConditionEvaluationError::new(format!(
                "no condition handler registered for `{}`",
                query.function()
            )));
        };

        match behavior {
            ConditionBehavior::Bool(value) => Ok(ConditionValue::Bool(*value)),
            ConditionBehavior::Enum(value) => Ok(ConditionValue::EnumVariant(value.clone())),
            ConditionBehavior::Failure(reason) => {
                Err(ConditionEvaluationError::new(reason.clone()))
            }
        }
    }
}

fn schema_fingerprint_from_token(
    token: Option<&str>,
    schema: Option<&recite_core::ProjectSchema>,
) -> (SchemaFingerprint, String) {
    if let Some(token) = token {
        let fingerprint = canonical_source_fingerprint(token);
        return (
            SchemaFingerprint::Fingerprint(fingerprint),
            token.to_owned(),
        );
    }
    if let Some(schema) = schema {
        return (
            schema.canonical_fingerprint(),
            "schema_fixture_canonical".to_owned(),
        );
    }
    (SchemaFingerprint::NoSchema, "no_schema".to_owned())
}

fn fingerprint_label(fingerprint: &ContentFingerprint) -> String {
    format!(
        "{}:{}",
        fingerprint.algorithm().as_str(),
        to_hex(fingerprint.digest().as_bytes())
    )
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn schema_fingerprint_snapshot(
    schema_fingerprint: &SchemaFingerprint,
) -> DialogueSchemaFingerprintSnapshot {
    match schema_fingerprint {
        SchemaFingerprint::NoSchema => DialogueSchemaFingerprintSnapshot::NoSchema,
        SchemaFingerprint::Fingerprint(fingerprint) => {
            DialogueSchemaFingerprintSnapshot::Fingerprint(DialogueContentFingerprintSnapshot {
                algorithm: fingerprint.algorithm().as_str().to_owned(),
                digest: fingerprint.digest().as_bytes().to_vec(),
            })
        }
    }
}
