//! Deterministic Recite runtime with no engine dependencies.

mod error;
mod event;
mod session;
mod traversal;

pub use error::{DialogueError, UnsupportedStatementKind};
pub use event::{ChoiceEchoMode, DialogueChoice, DialogueEvent, DialogueLine};
pub use session::DialogueSession;
pub use traversal::{choose, next, start_scene};
