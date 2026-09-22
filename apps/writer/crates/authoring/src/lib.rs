//! Source-preserving authoring sessions for the native Recite writer.
//! Parsing, validation, IDs, compilation, and preview use the real Recite crates.

mod document;
mod edits;
mod examples;
mod flow;
mod preview;
mod projection;
mod recovery;
mod script;
mod structure;
mod workbench;

pub use document::{Document, EditError, ProjectContext};
pub use examples::{WRITER_EXAMPLES, WriterExample};
pub use flow::{SceneLink, scene_links};
pub use preview::{Preview, PreviewError, PreviewPage, PreviewSetup};
pub use projection::{Passage, PassageKind};
pub use recovery::RecoveredDraft;
pub use script::{ScriptBlock, ScriptEntry};
pub use workbench::{View, Workbench, WorkbenchError};

pub const FIXTURE: &str =
    include_str!("../../../../../fixtures/recite/valid/gui_bakeoff/crossroads.recite");
pub const DOCUMENT_NAME: &str = "crossroads.recite";

mod history;
mod projection_cache;

mod search;
pub use search::{SearchHit, SearchIndex};

#[cfg(feature = "benchmarks")]
pub mod workload;

pub use recite_runtime::{ConditionExpectedType, ConditionValue, EffectAck};

mod completion;
pub use completion::SourceCompletions;

mod rules;
pub use rules::{ReplyRules, RuleArgument, RuleEffect, RuleExpression};

mod rename;
pub use rename::{ProjectRename, RenameChange, rename_manifest_source};
