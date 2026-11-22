//! Intents a frontend raises from its controls, and read access to state that changes only
//! through named operations.
use super::*;

/// Intents a frontend raises from controls; each keeps related state consistent.
impl Writer {
    /// Ask the shell to open a search.
    pub fn request_search(&mut self, search: shell::Search) {
        self.app_request = Some(shell::Request::Search(search));
    }
    /// Ask the shell to show the project welcome screen.
    pub fn request_projects(&mut self) {
        self.app_request = Some(shell::Request::Projects);
    }
    /// Ask the shell to open settings.
    pub fn request_settings(&mut self) {
        self.app_request = Some(shell::Request::Settings);
    }
    /// Close document actions after a chosen action.
    pub fn close_document_menu(&mut self) {
        self.document_menu = attention::DocumentMenu::Closed;
    }
    /// Open or close document actions from their toolbar control, Workspace. While the Settings
    /// sheet is open the control is its toggle instead, and closes it.
    pub fn toggle_document_menu(&mut self) {
        if self.settings_sheet {
            self.app_request = Some(shell::Request::CloseSettings);
        } else {
            self.document_menu.toggle();
        }
    }
    /// Dismiss document actions without choosing one.
    pub fn dismiss_document_menu(&mut self) {
        self.document_menu = attention::DocumentMenu::Dismissed;
    }
    /// Report that a copied link reached the system clipboard.
    pub fn clipboard_written(&mut self) {
        self.status = "Link sent to clipboard.".into();
    }
    /// Resume background work that a native message may have unblocked.
    pub fn after_native_message(&mut self) {
        self.refresh_map_if_needed();
        self.resume_external_notices();
        self.resume_linked_editor();
        self.check_prose_if_needed();
    }
}

impl Writer {
    /// The open document and whether it has unsaved edits.
    pub fn document_caption(&self) -> String {
        format!(
            "{}{}",
            self.session.model().document().key(),
            if self.session.current_dirty() {
                " · Unsaved"
            } else {
                ""
            }
        )
    }
}

/// Read access to state whose changes go through named operations.
impl Writer {
    /// Whether the window is writing, confirming a close or exit, or closing.
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn document_menu(&self) -> attention::DocumentMenu {
        self.document_menu
    }
    /// Whether the shell's Settings sheet hangs over this window: everything but the toolbar's
    /// Workspace control, the sheet's toggle, is inert behind it.
    pub fn settings_sheet(&self) -> bool {
        self.settings_sheet
    }
    /// Whether the choice slip hanging from `opener` is open.
    pub fn choosing(&self, opener: &Opener) -> bool {
        self.choosing.as_ref() == Some(opener)
    }
}
