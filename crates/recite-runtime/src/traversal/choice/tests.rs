use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{
        BlockIndex, CompiledAssetId, CompiledDialogue, CompiledDivertTarget, CompilerVersion,
        SchemaFingerprint, SourceMapId, StatementIndex, StatementRange,
        canonical_compiled_dialogue_fingerprint,
    },
};

use crate::session::{PendingPrompt, PendingPromptChoice, SessionPhase};
use crate::{
    ChoiceAvailability, ChoiceAvailabilityReason, DialogueError, DialogueSession,
    DialogueSessionOptions, EmptyDialogueContext,
};

use super::choose;

#[test]
fn unavailable_pending_choice_is_structured_error_without_mutating_session() {
    let asset = compiled_asset();
    let choice_id = ChoiceId::new("locked_choice").expect("valid choice ID");
    let mut session = DialogueSession::new(
        &asset.header,
        asset.sources.clone(),
        BlockIndex::new(0),
        StatementRange::new(StatementIndex::new(0), 0),
        canonical_compiled_dialogue_fingerprint(&asset).expect("valid test fingerprint"),
        DialogueSessionOptions::default(),
    );
    session.phase = SessionPhase::AwaitingChoice(PendingPrompt {
        statement: StatementIndex::new(0),
        choices: vec![PendingPromptChoice {
            id: choice_id.clone(),
            target: CompiledDivertTarget::End,
            is_available: false,
            availability: missing_trust_availability(),
        }],
    });

    assert_eq!(
        choose(
            &asset,
            &mut session,
            choice_id.clone(),
            &EmptyDialogueContext
        ),
        Err(DialogueError::UnavailableChoice {
            choice: choice_id.clone(),
            availability: Box::new(missing_trust_availability()),
        })
    );
    assert_eq!(
        match &session.phase {
            SessionPhase::AwaitingChoice(prompt) => Some(prompt.choice_ids()),
            _ => None,
        },
        Some(vec![choice_id])
    );
    assert!(session.selected_choice_history().is_empty());
}

#[test]
fn unavailable_choice_display_preserves_reason_formatting() {
    let choice = ChoiceId::new("locked_choice").expect("valid choice ID");

    assert_eq!(
        DialogueError::UnavailableChoice {
            choice: choice.clone(),
            availability: Box::new(ChoiceAvailability::unavailable(None, None)),
        }
        .to_string(),
        "choice `locked_choice` is unavailable"
    );
    assert_eq!(
        DialogueError::UnavailableChoice {
            choice,
            availability: Box::new(missing_trust_availability()),
        }
        .to_string(),
        "choice `locked_choice` is unavailable: missing trust"
    );
}

fn missing_trust_availability() -> ChoiceAvailability {
    ChoiceAvailability::unavailable(
        Some(ChoiceAvailabilityReason {
            id: recite_core::AvailabilityReasonId::new("missing_trust").expect("valid reason id"),
            source_text: "missing trust".to_owned(),
            text: "missing trust".to_owned(),
            origin: None,
            args: Vec::new(),
        }),
        None,
    )
}

fn compiled_asset() -> CompiledDialogue {
    compile_inputs(
        [CompileInput::new(
            "dialogue/start.recite",
            ":: start default\n> line@12345678901234567890\n  Line.\n-> END\n",
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("test source compiles")
    .asset
    .expect("asset emitted")
    .dialogue
}
