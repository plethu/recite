//! Native context snapshots and command authority have distinct freshness boundaries.
use super::*;

#[derive(PartialEq)]
pub(super) struct Scope {
    pub(super) writer: Option<DocumentScope>,
    pub(super) welcome: bool,
    pub(super) settings: bool,
    pub(super) picker: Option<(picker::PickerInstance, picker::Mode, picker::PickerRevision)>,
    pub(super) keymap: recite_config::Keymap,
}
pub(super) struct DocumentScope {
    window: WindowId,
    view: recite_writer_model::View,
    workspace: crate::map::Workspace,
    stamp: recite_writer_model::AnalysisStamp,
    pub(super) attention: crate::attention::Attention,
    pub(super) editing: Option<EditingTarget>,
    pub(super) field_restoration: Option<FieldRestoration>,
    pub(super) writing: bool,
    pub(super) screen: crate::workspaces::Screen,
    pub(super) document_menu: DocumentMenu,
}
impl PartialEq for DocumentScope {
    fn eq(&self, other: &Self) -> bool {
        self.same_command_scope(other)
            && self.attention == other.attention
            && self.field_restoration == other.field_restoration
    }
}
impl DocumentScope {
    fn same_command_scope(&self, other: &Self) -> bool {
        self.window == other.window
            && self.view == other.view
            && self.workspace == other.workspace
            && self.stamp == other.stamp
            && self.attention.same_command_scope(&other.attention)
            && self.editing == other.editing
            && self.writing == other.writing
            && self.screen == other.screen
            && self.document_menu == other.document_menu
    }
}
impl Scope {
    pub(super) fn same_command_scope(&self, other: &Self) -> bool {
        self.welcome == other.welcome
            && self.settings == other.settings
            && self.picker == other.picker
            && self.keymap == other.keymap
            && match (&self.writer, &other.writer) {
                (Some(writer), Some(other)) => writer.same_command_scope(other),
                (None, None) => true,
                _ => false,
            }
    }
    pub(super) fn capture(app: &App) -> Self {
        Self {
            writer: app.writer.as_ref().map(|writer| {
                let model = writer.session.model();
                DocumentScope {
                    window: writer.window,
                    view: model.view().clone(),
                    workspace: writer.map.workspace,
                    stamp: model.document().analysis_stamp(),
                    attention: writer.attention.clone(),
                    editing: EditingTarget::capture(writer),
                    field_restoration: writer.field_restoration(),
                    writing: writer.phase == Phase::Writing,
                    screen: writer.workspaces.screen,
                    document_menu: writer.document_menu,
                }
            }),
            welcome: app.show_welcome,
            settings: app.settings_open,
            keymap: app.preferences.config.ui.keymap,
            picker: app
                .picker
                .as_ref()
                .map(|picker| (picker.instance.clone(), picker.mode(), picker.revision)),
        }
    }
}
