//! Focus overlays the selected workspace; native pane requests carry their own identity.
use crate::{Phase, Writer, map::Workspace};
use recite_core::DocumentKey;
use recite_writer_model::View;
use std::sync::Arc;
use xilem::WindowId;
use xilem::masonry::core::{Widget, WidgetId, WidgetRef};
use xilem::masonry::{core, widgets};

/// The accepted passage whose inline field may own the product draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditingTarget {
    window: WindowId,
    document: DocumentKey,
    passage: String,
}

/// A refused native admission must either restore its admitted passage or leave prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldRestoration {
    Clear,
    Passage(EditingTarget),
}
/// An explicit request to reveal/focus a passage, distinct from its draft authority.
#[derive(Clone, Debug)]
pub(crate) struct EditRequest(Arc<EditingTarget>);
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

#[derive(Clone, Default, PartialEq)]
pub(crate) struct PassageField(pub Option<EditingTarget>);
impl PassageField {
    pub fn focused(
        mut root: WidgetRef<'_, dyn Widget>,
        focused: WidgetId,
    ) -> Option<EditingTarget> {
        loop {
            if let Some(input) = root.downcast::<widgets::TextInput>() {
                return (input.area_pod().id() == focused)
                    .then(|| input.get_prop::<Self>().0.clone())
                    .flatten();
            }
            root = root
                .children()
                .into_iter()
                .find(|child| child.ctx().has_focus_target())?;
        }
    }
}
impl core::UsesProperty<PassageField> for widgets::TextInput {}
impl core::Property for PassageField {
    fn static_default() -> &'static Self {
        static OTHER: PassageField = PassageField(None);
        &OTHER
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
pub(crate) enum PaneMotion {
    To(Pane),
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Pane {
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
pub(crate) enum DocumentMenu {
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
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Region(pub Option<Pane>);
/// A contextual pane's preferred entry, without changing its geometry or Tab order.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct PaneEntry(pub bool);
impl core::UsesProperty<PaneEntry> for widgets::Flex {}
impl core::Property for PaneEntry {
    fn static_default() -> &'static Self {
        static NONE: PaneEntry = PaneEntry(false);
        &NONE
    }
}
impl core::UsesProperty<Region> for widgets::SizedBox {}
impl core::UsesProperty<Region> for widgets::Flex {}
impl core::Property for Region {
    fn static_default() -> &'static Self {
        static NONE: Region = Region(None);
        &NONE
    }
}
#[derive(Clone, Debug)]
pub(crate) struct PaneRequest(Arc<PaneMotion>);
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
pub(crate) struct Attention {
    focused: bool,
    pane: Option<PaneRequest>,
    field: Option<EditingTarget>,
    edit_request: Option<EditRequest>,
}
impl Attention {
    pub fn same_command_scope(&self, other: &Self) -> bool {
        self.focused == other.focused && self.pane == other.pane
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn leave_focus(&mut self) {
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
    pub fn clear_edit_request(&mut self) {
        self.edit_request = None;
    }
    pub fn request_edit(&mut self, target: EditingTarget) {
        self.edit_request = Some(EditRequest(Arc::new(target)));
    }
}
impl Writer {
    pub(crate) fn editing_passage(&self, passage: &str) -> bool {
        self.attention
            .field()
            .is_some_and(|target| target.belongs_to(self) && target.passage == passage)
    }
    pub(crate) fn request_editing(&mut self) {
        if let Some(target) = EditingTarget::capture(self) {
            self.attention.request_edit(target);
        }
    }
    pub(crate) fn visible_workspace(&self) -> Workspace {
        if self.attention.focused {
            Workspace::Script
        } else {
            self.map.workspace
        }
    }
    pub(crate) fn toggle_focus(&mut self) {
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
    pub(crate) fn focus_pane(&mut self, direction: PaneMotion) {
        self.attention.pane = Some(PaneRequest(Arc::new(direction)));
    }
}

#[cfg(test)]
mod tests;
