//! Deterministic Recite runtime with no engine dependencies.
//!
//! The runtime consumes compiled dialogue assets from `recite-compiler`, keeps
//! session state separate from game state, evaluates conditions through a
//! caller-provided context, and emits structured events for lines, prompts,
//! effects, and scene completion.
//!
//! Runtime traversal is deliberately side-effect free: effects are requests that
//! the host observes and handles, then acknowledges when required. Session
//! snapshots are structural save data and should be stored or authenticated by
//! the host save system.
//!
//! Adapter-facing convenience APIs are intentionally deferred to future adapter
//! crates. Until those crates exist, adapters may call this crate directly while
//! preserving the host-agnostic contract in `docs/engine-adapter-contract.md`.
//! Broader game-developer workflow guides live in the [docs site][guides] as
//! they are filled in.
//!
//! [guides]: https://github.com/plethu/recite/tree/main/docs-site/src/content/docs
//!
//! # Example: Start A Session And Handle Events
//!
//! ```
//! # fn compile_asset() -> Result<recite_core::CompiledDialogue, Box<dyn std::error::Error>> {
//! #     use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
//! #     use recite_core::{
//! #         CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
//! #     };
//! #     let source = concat!(
//! #         ":: start default\n",
//! #         "> intro_001@8843fd6f53f020a12b31\n",
//! #         "  Hello.\n",
//! #         "-> END\n",
//! #     );
//! #     let options = CompileOptions::new(
//! #         CompilerVersion::new("0.0.1")?,
//! #         CompiledAssetId::new("example-dialogue")?,
//! #         SourceMapId::new("example-source-map")?,
//! #         SchemaFingerprint::NoSchema,
//! #     );
//! #     let report = compile_inputs(
//! #         [CompileInput::new("dialogue/start.recite", source)],
//! #         options,
//! #     )?;
//! #     Ok(report.asset.expect("valid source emits an asset").dialogue)
//! # }
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_runtime::{
//!     DialogueEvent, EmptyDialogueContext, next, start_scene,
//! };
//!
//! let asset = compile_asset()?;
//! let mut session = start_scene(&asset, None)?;
//! let event = next(&asset, &mut session, &EmptyDialogueContext)?;
//!
//! match event {
//!     DialogueEvent::Line(line) => {
//!         assert_eq!(line.id.as_str(), "8843fd6f53f020a12b31");
//!         assert_eq!(line.text, "Hello.");
//!     }
//!     other => panic!("expected line event, got {other:?}"),
//! }
//! # Ok(())
//! # }
//! ```

mod context;
mod error;
mod event;
mod locale;
mod session;
mod session_serialization;
mod session_snapshot;
mod traversal;

pub use context::{
    ConditionArgument, ConditionArguments, ConditionEvaluationError, ConditionExpectedType,
    ConditionQuery, ConditionValue, DialogueContext, EmptyDialogueContext,
};
pub use error::DialogueError;
pub use event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
    ChoiceEchoMode, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueEvent, DialogueLine, DialoguePlural, DialoguePluralResolution,
    DialoguePluralResolutionOutcome, EffectAck,
};
pub use locale::{
    InterpolationValueProvider, InterpolationValues, LocaleError, LocaleProvider, PluralResolution,
    PluralResolutionAttempt, PluralResolutionOutcome, TextDomain,
};
pub use session::{DialogueSession, DialogueSessionOptions};
pub use session_serialization::{
    decode_session_messagepack, encode_session_messagepack, restore_session,
};
pub use session_snapshot::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueChoiceAvailabilityReasonArgSnapshot,
    DialogueChoiceAvailabilityReasonOriginSnapshot, DialogueChoiceAvailabilityReasonSnapshot,
    DialogueChoiceAvailabilityReasonTreeSnapshot, DialogueChoiceAvailabilityReasonValueSnapshot,
    DialogueChoiceAvailabilitySnapshot, DialogueContentFingerprintSnapshot,
    DialogueDeferredEffectSnapshot, DialogueSchemaFingerprintSnapshot,
    DialogueSessionFrameSnapshot, DialogueSessionPendingChoiceSnapshot,
    DialogueSessionPendingEffectSnapshot, DialogueSessionPendingPromptSnapshot,
    DialogueSessionRangeSnapshot, DialogueSessionSnapshot, DialogueSessionSnapshotConversionError,
    DialogueSessionSourceSnapshot, SESSION_SNAPSHOT_FORMAT_VERSION_V0,
    SESSION_SNAPSHOT_FORMAT_VERSION_V1, snapshot_session,
};
pub use traversal::{
    DialogueTrace, LocaleResolution, PluralLineTrace, acknowledge_effect, choose, choose_with,
    next, next_with, start_scene, start_scene_with_options,
};
