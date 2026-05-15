//! Deterministic Recite runtime with no engine dependencies.

mod context;
mod error;
mod event;
mod session;
mod traversal;

pub use context::{
    ConditionArgument, ConditionArguments, ConditionEvaluationError, ConditionQuery,
    DialogueContext, EmptyDialogueContext,
};
pub use error::{DialogueError, UnsupportedStatementKind};
pub use event::{ChoiceEchoMode, DialogueChoice, DialogueEvent, DialogueLine};
pub use session::DialogueSession;
pub use traversal::{choose, next, start_scene};
