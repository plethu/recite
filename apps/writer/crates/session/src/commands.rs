//! Shared command wording for menus, search and personal shortcuts.
pub mod keybindings;
pub mod targets;
use crate::messages::{MsgId, text};
use recite_config::WriterCommand;

pub fn vim_alias(query: &str) -> Option<WriterCommand> {
    match query.trim().trim_start_matches(':') {
        "w" => Some(WriterCommand::Save),
        "wa" => Some(WriterCommand::SaveAll),
        "q" => Some(WriterCommand::Close),
        _ => None,
    }
}

pub fn label(command: WriterCommand, focused: bool) -> String {
    match command {
        WriterCommand::Script => text(MsgId::WriterWorkspaceScriptView),
        WriterCommand::Map => text(MsgId::WriterWorkspaceMapView),
        WriterCommand::Source => text(MsgId::WriterWorkspaceSourceView),
        WriterCommand::Split => text(MsgId::WriterWorkspaceSplitView),
        WriterCommand::Focus if focused => text(MsgId::WriterWorkspaceExitFocus),
        WriterCommand::Focus => text(MsgId::WriterWorkspaceFocusWriting),
        WriterCommand::Commands => text(MsgId::WriterWorkspaceCommands),
        WriterCommand::GoTo => text(MsgId::WriterWorkspaceGoTo),
        WriterCommand::OpenProject => text(MsgId::WriterGuiOpenProject),
        WriterCommand::Save => text(MsgId::WriterWorkspaceSave),
        WriterCommand::SaveAll => text(MsgId::WriterWorkspaceSaveAll),
        WriterCommand::Apply => text(MsgId::WriterApplyDraft),
        WriterCommand::Undo => text(MsgId::WriterWorkspaceUndo),
        WriterCommand::Redo => text(MsgId::WriterWorkspaceRedo),
        WriterCommand::AddBeat => text(MsgId::WriterGuiAddBeat),
        WriterCommand::AddLine => text(MsgId::WriterGuiAddLine),
        WriterCommand::AddReply => text(MsgId::WriterGuiAddReply),
        WriterCommand::Preview => text(MsgId::WriterGuiTryScene),
        WriterCommand::Localise => text(MsgId::WriterLocalise),
        WriterCommand::Declarations => text(MsgId::WriterDeclarations),
        WriterCommand::Build => text(MsgId::WriterBuildScenes),
        WriterCommand::Rename => text(MsgId::WriterRenameProject),
        WriterCommand::Settings => text(MsgId::WriterGuiUserPreferences),

        WriterCommand::Back => "Back".into(),
        WriterCommand::Forward => "Forward".into(),
        WriterCommand::Find => "Find scene or beat".into(),
        WriterCommand::NextMatch => "Next search match".into(),
        WriterCommand::PreviousMatch => "Previous search match".into(),
        WriterCommand::PaneLeft => "Focus pane left".into(),
        WriterCommand::PaneRight => "Focus pane right".into(),
        WriterCommand::PaneUp => "Focus pane above".into(),
        WriterCommand::PaneDown => "Focus pane below".into(),
        WriterCommand::Close => "Close application".into(),
    }
}
