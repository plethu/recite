//! One command vocabulary for visible actions, search and workspace shortcuts.
mod keyboard;
mod vim;
pub(crate) use vim::{Navigation, modifier_key as vim_modifier_key};
mod shortcuts;
pub(crate) use shortcuts::{Hints, hint, shortcut};
mod palette;
pub(crate) use keyboard::keyboard;
pub(crate) use vim::keyboard as vim_keyboard;
mod execute;
mod targets;
use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) use palette::Palette;

pub(crate) use recite_config::WriterCommand as Command;
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
pub(crate) trait CommandExt {
    fn label(self, writer: Writer) -> String;
    fn shortcut(self, writer: Writer) -> String;
    fn enabled(self, writer: Writer) -> bool;
    fn button(self, writer: Writer) -> crate::design::Button;
    fn run(self, writer: Writer);
}
impl CommandExt for Command {
    fn run(self, writer: Writer) {
        execute::run(self, writer);
    }
    fn label(self, writer: Writer) -> String {
        match self {
            Self::Script => text(MsgId::WriterWorkspaceScriptView),
            Self::Map => text(MsgId::WriterWorkspaceMapView),
            Self::Source => text(MsgId::WriterWorkspaceSourceView),
            Self::Split => text(MsgId::WriterWorkspaceSplitView),
            Self::Focus if *writer.layout.focus.read() => text(MsgId::WriterWorkspaceExitFocus),
            Self::Focus => text(MsgId::WriterWorkspaceFocusWriting),
            Self::Commands => text(MsgId::WriterWorkspaceCommands),
            Self::GoTo => text(MsgId::WriterWorkspaceGoTo),
            Self::OpenProject => text(MsgId::WriterGuiOpenProject),
            Self::Save => text(MsgId::WriterWorkspaceSave),
            Self::SaveAll => text(MsgId::WriterWorkspaceSaveAll),
            Self::Apply => text(MsgId::WriterApplyDraft),
            Self::Undo => text(MsgId::WriterWorkspaceUndo),
            Self::Redo => text(MsgId::WriterWorkspaceRedo),
            Self::AddBeat => text(MsgId::WriterGuiAddBeat),
            Self::AddLine => text(MsgId::WriterGuiAddLine),
            Self::AddReply => text(MsgId::WriterGuiAddReply),
            Self::Preview => text(MsgId::WriterGuiTryScene),
            Self::Localise => text(MsgId::WriterLocalise),
            Self::Declarations => text(MsgId::WriterDeclarations),
            Self::Build => text(MsgId::WriterBuildScenes),
            Self::Rename => text(MsgId::WriterRenameProject),
            Self::Settings => text(MsgId::WriterGuiUserPreferences),

            Self::Back => "Back".into(),
            Self::Forward => "Forward".into(),
            Self::Find => "Find scene or beat".into(),
            Self::NextMatch => "Next search match".into(),
            Self::PreviousMatch => "Previous search match".into(),
            Self::PaneLeft => "Focus pane left".into(),
            Self::PaneRight => "Focus pane right".into(),
            Self::PaneUp => "Focus pane above".into(),
            Self::PaneDown => "Focus pane below".into(),
            Self::Close => "Close application".into(),
        }
    }
    fn shortcut(self, writer: Writer) -> String {
        if self == Self::Apply {
            return crate::design::keyboard::submit_hint().into();
        }
        writer
            .preferences
            .read()
            .config
            .writer
            .shortcuts
            .binding(self)
            .replace(
                "Primary",
                if cfg!(target_os = "macos") {
                    "Cmd"
                } else {
                    "Ctrl"
                },
            )
    }
    fn enabled(self, writer: Writer) -> bool {
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
    fn button(self, writer: Writer) -> crate::design::Button {
        crate::design::Button::new()
            .flat()
            .enabled(self.enabled(writer))
            .named(self.label(writer))
            .shortcut(self.shortcut(writer))
            .on_press(move |_| self.run(writer))
            .child(self.label(writer))
    }
}
