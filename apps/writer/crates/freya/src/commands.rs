//! One command vocabulary for visible actions, search and workspace shortcuts.
mod keyboard;
mod palette;
pub(crate) use keyboard::keyboard;
mod execute;
mod targets;
use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) use palette::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Script,
    Map,
    Source,
    Split,
    Focus,
    Commands,
    GoTo,
    OpenProject,
    Save,
    SaveAll,
    Apply,
    Undo,
    Redo,
    AddBeat,
    AddLine,
    AddReply,
    Preview,
    Localise,
    Declarations,
    Build,
    Rename,
    Settings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchMode {
    Commands,
    GoTo,
}
#[derive(Clone, Copy)]
pub(crate) struct Search {
    pub mode: State<Option<SearchMode>>,
    pub return_focus: State<AccessibilityId>,
}
impl Search {
    pub fn new() -> Self {
        Self {
            mode: use_state(|| None),
            return_focus: use_state(|| *Platform::get().focused_accessibility_id.peek()),
        }
    }
    pub fn open(mut self, mode: SearchMode) {
        self.return_focus
            .set(*Platform::get().focused_accessibility_id.peek());
        self.mode.set(Some(mode));
    }
    pub fn close(mut self) {
        self.mode.set(None);
        self.return_focus.peek().request_focus();
    }
}
impl Command {
    pub const ALL: &[Self] = &[
        Self::Script,
        Self::Map,
        Self::Source,
        Self::Split,
        Self::Focus,
        Self::GoTo,
        Self::OpenProject,
        Self::Save,
        Self::SaveAll,
        Self::Apply,
        Self::Undo,
        Self::Redo,
        Self::AddBeat,
        Self::AddLine,
        Self::AddReply,
        Self::Preview,
        Self::Localise,
        Self::Declarations,
        Self::Build,
        Self::Rename,
        Self::Settings,
    ];
    pub fn label(self, writer: Writer) -> String {
        text(match self {
            Self::Script => MsgId::WriterWorkspaceScriptView,
            Self::Map => MsgId::WriterWorkspaceMapView,
            Self::Source => MsgId::WriterWorkspaceSourceView,
            Self::Split => MsgId::WriterWorkspaceSplitView,
            Self::Focus if *writer.layout.focus.read() => MsgId::WriterWorkspaceExitFocus,
            Self::Focus => MsgId::WriterWorkspaceFocusWriting,
            Self::Commands => MsgId::WriterWorkspaceCommands,
            Self::GoTo => MsgId::WriterWorkspaceGoTo,
            Self::OpenProject => MsgId::WriterGuiOpenProject,
            Self::Save => MsgId::WriterWorkspaceSave,
            Self::SaveAll => MsgId::WriterWorkspaceSaveAll,
            Self::Apply => MsgId::WriterApplyDraft,
            Self::Undo => MsgId::WriterWorkspaceUndo,
            Self::Redo => MsgId::WriterWorkspaceRedo,
            Self::AddBeat => MsgId::WriterGuiAddBeat,
            Self::AddLine => MsgId::WriterGuiAddLine,
            Self::AddReply => MsgId::WriterGuiAddReply,
            Self::Preview => MsgId::WriterGuiTryScene,
            Self::Localise => MsgId::WriterLocalise,
            Self::Declarations => MsgId::WriterDeclarations,
            Self::Build => MsgId::WriterBuildScenes,
            Self::Rename => MsgId::WriterRenameProject,
            Self::Settings => MsgId::WriterGuiUserPreferences,
        })
    }
    pub fn shortcut(self) -> String {
        let primary = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        match self {
            Self::Commands => format!("{primary}+Shift+P"),
            Self::GoTo => format!("{primary}+P"),
            Self::Save => format!("{primary}+S"),
            Self::Settings => format!("{primary}+,"),
            Self::Apply => format!("{primary}+Enter"),
            _ => String::new(),
        }
    }
    pub fn enabled(self, writer: Writer) -> bool {
        if matches!(
            self,
            Self::Save | Self::SaveAll | Self::Declarations | Self::Build | Self::Rename
        ) && writer.files.read().is_none()
        {
            return false;
        }
        if self == Self::OpenProject {
            return writer.layout.project_controls.read().is_some();
        }
        if self == Self::Apply {
            return writer
                .buffers
                .model
                .read()
                .as_ref()
                .is_ok_and(|m| m.has_draft());
        }
        if matches!(self, Self::AddLine | Self::AddReply) {
            return writer
                .buffers
                .model
                .read()
                .as_ref()
                .is_ok_and(|m| m.selected_block().ok().flatten().is_some());
        }
        true
    }
    pub fn button(self, writer: Writer) -> crate::design::Button {
        crate::design::Button::new()
            .flat()
            .enabled(self.enabled(writer))
            .named(self.label(writer))
            .on_press(move |_| self.run(writer))
            .child(self.label(writer))
    }
}

pub(crate) fn shortcut(event: &KeyboardEventData) -> Option<Command> {
    let primary = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    match (event.code, event.modifiers) {
        (Code::KeyP, mods) if mods == primary | Modifiers::SHIFT => Some(Command::Commands),
        (Code::KeyP, mods) if mods == primary => Some(Command::GoTo),
        (Code::KeyS, mods) if mods == primary => Some(Command::Save),
        (Code::Comma, mods) if mods == primary => Some(Command::Settings),
        _ => None,
    }
}
