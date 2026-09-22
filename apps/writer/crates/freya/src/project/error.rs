use std::io;
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error(transparent)]
    Discovery(#[from] recite_config::ProjectDiscoveryError),
    #[error(
        "Project discovery is incomplete. Repair the project's discovery diagnostics before opening it here."
    )]
    Incomplete,
    #[error("Project validation failed: {}", crate::project_context::diagnostic_messages(.0))]
    Validation(Vec<recite_core::Diagnostic>),
    #[error(
        "This document is already open in another writer. Close it there before opening it here."
    )]
    RecoveryInUse,
    #[error("The recovery snapshot uses an unsupported version; it has been left untouched.")]
    RecoveryVersion,
    #[error("Recovery snapshot could not be read or written: {0}")]
    Recovery(#[from] serde_json::Error),
    #[error(transparent)]
    Workbench(#[from] recite_writer_model::WorkbenchError),
    #[error("The project has no Recite source files.")]
    Empty,
    #[error(
        "A document changed after rename review. Review the rename again, or undo subsequent edits first."
    )]
    StaleRename,
    #[error(
        "This project has no schema. Configure its generated schema in recite.project.toml first."
    )]
    NoSchema,
    #[error(
        "Choose a standalone TOML source with the same producer identity as this generated schema."
    )]
    SchemaOwnership,
    #[error("Save this document before closing it. Your draft remains open.")]
    UnsavedDocument,
    #[error("The selected file is no longer in this project.")]
    Selection,
    #[error(
        "This file changed on disk. Your edits remain open. Compare the versions before saving."
    )]
    Conflict,
    #[error("Refusing to replace a symbolic link or a non-regular file.")]
    FileKind,
    #[error("Another editor is saving this file, or its save lock could not be opened: {0}")]
    Locked(io::Error),
    #[error(
        "Replacement could not be confirmed: {0}. Keep your draft and reload the disk version through the recovery-copy action before retrying."
    )]
    Commit(io::Error),
    #[error("Background recovery failed: {0}")]
    BackgroundRecovery(String),
    #[error("File operation failed: {0}")]
    Io(#[from] io::Error),
}
