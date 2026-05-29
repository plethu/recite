use recite_core::{
    BlockIndex, BlockLookupTable, ChoiceId, ChoiceLookupTable, CompiledAssetHeader,
    CompiledAssetId, CompiledDialogue, CompiledDivertTarget, CompilerVersion, LineLookupTable,
    SchemaFingerprint, SourceMapId, StatementIndex, StatementRange,
};

use crate::session::{PendingPrompt, PendingPromptChoice};
use crate::{DialogueError, DialogueSession, DialogueSessionOptions, EmptyDialogueContext};

use super::choose;

#[test]
fn unavailable_pending_choice_is_structured_error_without_mutating_session() {
    let asset = empty_asset();
    let choice_id = ChoiceId::new("locked_choice").expect("valid choice ID");
    let mut session = DialogueSession::new(
        &asset.header,
        asset.sources.clone(),
        BlockIndex::new(0),
        StatementRange::new(StatementIndex::new(0), 0),
        DialogueSessionOptions::default(),
    );
    session.pending_prompt = Some(PendingPrompt {
        statement: StatementIndex::new(0),
        choices: vec![PendingPromptChoice {
            id: choice_id.clone(),
            target: CompiledDivertTarget::End,
            is_available: false,
            unavailable_reason: Some("missing trust".to_owned()),
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
            reason: Some("missing trust".to_owned())
        })
    );
    assert_eq!(
        session
            .pending_prompt
            .as_ref()
            .map(PendingPrompt::choice_ids),
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
            reason: None,
        }
        .to_string(),
        "choice `locked_choice` is unavailable"
    );
    assert_eq!(
        DialogueError::UnavailableChoice {
            choice,
            reason: Some("missing trust".to_owned()),
        }
        .to_string(),
        "choice `locked_choice` is unavailable: missing trust"
    );
}

fn empty_asset() -> CompiledDialogue {
    CompiledDialogue {
        header: CompiledAssetHeader::messagepack_v0(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            SchemaFingerprint::NoSchema,
        ),
        default_block: BlockIndex::new(0),
        sources: Vec::new(),
        blocks: Vec::new(),
        statements: Vec::new(),
        match_arms: Vec::new(),
        lines: Vec::new(),
        choices: Vec::new(),
        speakers: Vec::new(),
        metadata: Vec::new(),
        effects: Vec::new(),
        source_maps: Vec::new(),
        block_lookup: BlockLookupTable::default(),
        line_lookup: LineLookupTable::default(),
        choice_lookup: ChoiceLookupTable::default(),
    }
}
