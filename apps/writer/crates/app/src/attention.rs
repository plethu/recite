//! Focus overlays the selected workspace; native pane requests carry their own identity.
pub use crate::WindowKey;
pub use crate::{Phase, Writer, map::Workspace};
use recite_core::DocumentKey;
use recite_writer_model::View;
use std::sync::Arc;
pub mod composition;
use composition::Composition;
pub use composition::Preedit;

/// The accepted passage whose inline field may own the product draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditingTarget {
    window: WindowKey,
    document: DocumentKey,
    passage: String,
}

/// A refused native admission must either restore its admitted passage or leave prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldRestoration {
    Clear,
    Passage(EditingTarget),
}
/// An explicit request to reveal/focus a passage, distinct from its draft authority.
#[derive(Clone, Debug)]
pub struct EditRequest(Arc<EditingTarget>);
impl EditRequest {
    pub fn target(&self) -> &EditingTarget {
        &self.0
    }
}
impl PartialEq for EditRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl EditingTarget {
    pub fn passage(writer: &Writer, passage: String) -> Self {
        Self {
            window: writer.window,
            document: writer.session.model().document().key().clone(),
            passage,
        }
    }
    pub fn belongs_to(&self, writer: &Writer) -> bool {
        self.window == writer.window && &self.document == writer.session.model().document().key()
    }
    pub fn view(&self) -> View {
        View::Passage(self.passage.clone())
    }
    pub fn capture(writer: &Writer) -> Option<Self> {
        let model = writer.session.model();
        let View::Passage(passage) = model.view() else {
            return None;
        };
        Some(Self::passage(writer, passage.clone()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneMotion {
    To(Pane),
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pane {
    Navigation,
    Map,
    Script,
    Preview,
    Declarations,
    Build,
    Settings,
    Rename,
    External,
    Localisation,
    DocumentActions,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocumentMenu {
    #[default]
    Closed,
    Open,
    Dismissed,
}
impl DocumentMenu {
    pub fn is_open(self) -> bool {
        self == Self::Open
    }
    pub fn toggle(&mut self) {
        *self = if self.is_open() {
            Self::Dismissed
        } else {
            Self::Open
        };
    }
}
#[derive(Clone, Debug)]
pub struct PaneRequest(Arc<PaneMotion>);
impl PartialEq for PaneRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl PaneRequest {
    pub fn motion(&self) -> PaneMotion {
        *self.0
    }
}
#[derive(Clone, Default, PartialEq)]
pub struct Attention {
    composition: Composition,
    focused: bool,
    pane: Option<PaneRequest>,
    field: Option<EditingTarget>,
    edit_request: Option<EditRequest>,
}
impl Attention {
    pub fn begin_preedit(&self) -> Preedit {
        self.composition.begin()
    }
    pub fn is_composing(&self) -> bool {
        self.composition.active()
    }
    pub fn same_command_scope(&self, other: &Self) -> bool {
        self.focused == other.focused && self.pane == other.pane
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub(crate) fn leave_focus(&mut self) {
        self.focused = false;
    }
    pub fn pane_request(&self) -> Option<&PaneRequest> {
        self.pane.as_ref()
    }
    pub fn field(&self) -> Option<&EditingTarget> {
        self.field.as_ref()
    }
    pub fn set_field(&mut self, field: Option<EditingTarget>) {
        self.field = field;
    }
    pub fn edit_request(&self) -> Option<&EditRequest> {
        self.edit_request.as_ref()
    }
    pub(crate) fn clear_edit_request(&mut self) {
        self.edit_request = None;
    }
    pub fn request_edit(&mut self, target: EditingTarget) {
        self.edit_request = Some(EditRequest(Arc::new(target)));
    }
}
impl Writer {
    pub(crate) fn require_finished_composition(&self) -> Result<(), String> {
        if self.attention.is_composing() {
            Err("Finish text composition before leaving this passage.".into())
        } else {
            Ok(())
        }
    }
    pub fn editing_passage(&self, passage: &str) -> bool {
        self.attention
            .field()
            .is_some_and(|target| target.belongs_to(self) && target.passage == passage)
    }
    pub fn request_editing(&mut self) {
        if let Some(target) = EditingTarget::capture(self) {
            self.attention.request_edit(target);
        }
    }
    pub fn visible_workspace(&self) -> Workspace {
        if self.attention.focused {
            Workspace::Script
        } else {
            self.map.workspace
        }
    }
    pub fn toggle_focus(&mut self) {
        if self.phase != Phase::Writing || self.work.busy() || self.navigation.busy() {
            self.status = "Finish the current action before changing writing focus.".into();
            return;
        }
        self.cancel_map_gesture();
        self.workspaces.screen = crate::workspaces::Screen::Write;
        self.attention.focused = !self.attention.focused;
        self.status = if self.attention.focused {
            "Writing focus"
        } else {
            "Returned to the workspace."
        }
        .into();
    }
    pub fn focus_pane(&mut self, direction: PaneMotion) {
        self.attention.pane = Some(PaneRequest(Arc::new(direction)));
    }
}
