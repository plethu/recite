//! Native context snapshots and command authority have distinct freshness boundaries.
use super::super::welcome::FormScope;
use super::*;
use recite_core::DocumentKey;

#[derive(PartialEq)]
pub struct Scope {
    pub(super) writer: Option<DocumentScope>,
    pub(super) welcome: bool,
    pub(super) project_form: Option<FormScope>,
    pub(super) settings: bool,
    pub(super) picker: Option<(picker::PickerInstance, picker::Mode, picker::PickerRevision)>,
    pub(super) keymap: recite_config::Keymap,
}
pub(super) struct DocumentScope {
    window: crate::WindowKey,
    document: DocumentKey,
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
    pub fn same_command_scope(&self, other: &Self) -> bool {
        self.same_picker_opening_scope(other) && self.stamp == other.stamp
    }
    pub fn same_picker_opening_scope(&self, other: &Self) -> bool {
        self.window == other.window
            && self.document == other.document
            && self.view == other.view
            && self.workspace == other.workspace
            && self.attention.same_command_scope(&other.attention)
            && self.editing == other.editing
            && self.writing == other.writing
            && self.screen == other.screen
            && self.document_menu == other.document_menu
    }
}
impl Scope {
    pub fn same_command_scope(&self, other: &Self) -> bool {
        self.same_shell(other)
            && match (&self.writer, &other.writer) {
                (Some(writer), Some(other)) => writer.same_command_scope(other),
                (None, None) => true,
                _ => false,
            }
    }
    pub fn same_picker_opening_scope(&self, other: &Self) -> bool {
        self.same_shell(other)
            && match (&self.writer, &other.writer) {
                (Some(writer), Some(other)) => writer.same_picker_opening_scope(other),
                (None, None) => true,
                _ => false,
            }
    }
    fn same_shell(&self, other: &Self) -> bool {
        self.welcome == other.welcome
            && self.project_form == other.project_form
            && self.settings == other.settings
            && self.picker == other.picker
            && self.keymap == other.keymap
    }
    pub fn capture(app: &Shell) -> Self {
        Self {
            writer: app.writer.as_ref().map(|writer| {
                let model = writer.session.model();
                DocumentScope {
                    window: writer.window,
                    document: model.document().key().clone(),
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
            project_form: (app.show_welcome && !app.settings_open).then(|| FormScope::capture(app)),
            settings: app.settings_open,
            keymap: app.preferences.config.ui.keymap,
            picker: app
                .picker
                .as_ref()
                .map(|picker| (picker.instance.clone(), picker.mode(), picker.revision)),
        }
    }
}
