//! Revision-independent snapshot fields.
//!
//! Table indices identify a position only within the exact compiled payload a
//! session was saved against. These fields identify the same position through
//! authored block, line and choice IDs and statement signatures, so a save can
//! be relocated in a patched asset. Effect requests are stored whole because
//! compiled effect IDs are derived from source positions.

use serde::{Deserialize, Serialize};

use super::DialogueContentFingerprintSnapshot;

/// Content and structure digests of the block a session was saved in.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueBlockDigestsSnapshot {
    pub content: DialogueContentFingerprintSnapshot,
    pub structure: DialogueContentFingerprintSnapshot,
}

/// The saved position, from the block's root range to the active range.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueResumeAnchorSnapshot {
    pub frames: Vec<DialogueResumeFrameSnapshot>,
    pub position: DialoguePositionAnchorSnapshot,
}

/// One entered `if` or `match` branch: where traversal resumes in the parent
/// range after the branch completes, and which branch was entered.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueResumeFrameSnapshot {
    pub position: DialoguePositionAnchorSnapshot,
    pub branch: DialogueBranchSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueBranchSnapshot {
    Then,
    Else,
    /// A match arm by pattern; `variant` is `None` for the wildcard arm.
    /// `occurrence` distinguishes repeated patterns in source order.
    Arm {
        variant: Option<String>,
        occurrence: u32,
    },
}

/// A position inside one statement range: after the latest statement with an
/// authored ID (`landmark`), followed by the statements in `trail`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialoguePositionAnchorSnapshot {
    pub landmark: Option<DialogueLandmarkSnapshot>,
    pub trail: Vec<DialogueStatementSignatureSnapshot>,
}

/// A statement identified by authored stable IDs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueLandmarkSnapshot {
    Line {
        id: String,
    },
    Prompt {
        line: Option<String>,
        choices: Vec<String>,
    },
}

/// A statement without an authored ID, identified by its behaviour.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueStatementSignatureSnapshot {
    Divert {
        target: Option<String>,
    },
    End,
    If {
        condition: Vec<u8>,
    },
    Match {
        scrutinee: Vec<u8>,
    },
    Effect {
        mode: DialogueEffectModeSnapshot,
        function: String,
        arguments: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueEffectModeSnapshot {
    Deferred,
    Immediate,
    Blocking,
}

/// A complete effect request, kept exactly as the runtime emitted or
/// collected it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueEffectRequestSnapshot {
    pub id: String,
    pub mode: DialogueEffectModeSnapshot,
    pub function: String,
    pub args: Vec<DialogueEffectArgumentSnapshot>,
    pub source_span: DialogueSourceSpanSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DialogueEffectArgumentSnapshot {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSourceSpanSnapshot {
    pub file: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end: Option<DialogueSourcePositionSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueSourcePositionSnapshot {
    pub line: u32,
    pub column: u32,
}
