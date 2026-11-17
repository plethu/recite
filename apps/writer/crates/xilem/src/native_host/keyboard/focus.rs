//! Native focus moves through RenderRoot's safe API; mounted fields retain their identities.
use super::*;
use crate::app::keyboard::CommandGeneration;
use crate::app::keyboard::{Action, DefaultFocus, Invocation, PickerAnchor};
use crate::app::picker;
use crate::attention::{EditRequest, EditingTarget};
use crate::localisation::{TranslationField, TranslationTarget};
use crate::presentation::marks::Wedge;
use crate::rules::view;
use xilem::masonry::accesskit;
use xilem::masonry::core::{Widget, WidgetRef};
use xilem::masonry::widgets::{Button, TextInput};

mod targets;
pub(super) use targets::renames;
pub(crate) use targets::slip_anchor;
use targets::{
    default_focus, fallback_focus, find, first_wedge, marked, outside_modal, place_picker,
    return_focus, slip_host,
};

#[derive(Clone, Default, PartialEq)]
pub(super) enum Kind {
    #[default]
    Other,
    Manuscript,
    Translation(TranslationTarget),
    Query,
    ActiveResult,
    ProjectForm,
    Text,
}
#[derive(Default)]
pub(super) struct Focus {
    picker: bool,
    /// The modal surface over the document, and once it has taken focus, the control to return
    /// to when it closes.
    modal: Option<Modal>,
    modal_return: Option<Option<WidgetId>>,
    /// The last request to focus the first problem, and whether it waits for its wedge to mount.
    problem: u64,
    problem_pending: bool,
    picker_instance: Option<picker::PickerInstance>,
    return_to: Option<WidgetId>,
    last: Option<WidgetId>,
    field: Option<(WidgetId, CommandGeneration, Kind)>,
    revealed: Option<CommandGeneration>,
    editing: Option<EditingTarget>,
    edit_request: Option<EditRequest>,
    /// Where an inline rename's slip was last told to hang.
    slip: Option<(u32, u32)>,
    /// Whether a control held focus after the last pass.
    held: bool,
    /// The control this host last moved focus to, while it still holds it.
    placed: Option<WidgetId>,
}
impl Focus {
    pub fn synchronize_manuscript(
        &mut self,
        root: &mut RenderRoot,
        editing: Option<&EditingTarget>,
        request: Option<&EditRequest>,
    ) {
        if self.editing.as_ref() != editing {
            if editing.is_none() && super::manuscript::focused(root).is_some() {
                root.focus_on(None);
            }
            self.editing = editing.cloned();
        }
        if self.edit_request.as_ref() == request {
            return;
        }
        let Some(target) = request
            .map(EditRequest::target)
            .filter(|target| Some(*target) == editing)
        else {
            self.edit_request = request.cloned();
            return;
        };
        let Some(field) = super::manuscript::find(root.get_layer_root(0), target) else {
            // Retry this explicit request after its field has mounted.
            return;
        };
        self.edit_request = request.cloned();
        super::panes::reveal_editor(root, field);
    }
    /// Move focus into a modal surface as it opens, and back to where it was when it closes.
    fn synchronize_modal(&mut self, root: &mut RenderRoot, context: &Context) {
        if self.modal == context.modal {
            // A modal mounted after its context: focus it once it has its default control.
            if self.modal.is_some() && self.modal_return.is_none() {
                self.enter_modal(root);
            } else if self.modal.is_some() && root.focused_widget().is_none() {
                // Its focused control went away, as a renamed heading's field gives way to its
                // slip: focus the new default, keeping where focus returns on close.
                if let Some(target) = default_focus(root) {
                    root.focus_on(Some(target));
                }
            }
            return;
        }
        match (self.modal, context.modal) {
            (None, Some(_)) => {
                self.modal_return = None;
                self.enter_modal(root);
            }
            (Some(_), None) => {
                if let Some(Some(target)) = self.modal_return.take()
                    && !root.focus_on(Some(target))
                {
                    // The control was rebuilt, as a renamed row is: focus its successor.
                    root.focus_on(return_focus(root));
                }
            }
            _ => self.enter_modal(root),
        }
        self.modal = context.modal;
    }
    /// Keep focus on a mounted modal surface: whatever moved it behind the surface this pass, as
    /// a manuscript field restored under a rename's slip, focus goes to the surface's default.
    pub fn hold_modal(&mut self, root: &mut RenderRoot, context: &Context) {
        if context.modal.is_none() {
            return;
        }
        if root
            .focused_widget()
            .is_some_and(|focused| outside_modal(root, focused))
            && let Some(target) = default_focus(root)
        {
            root.focus_on(Some(target));
        }
    }
    /// Hang an inline rename's slip from the heading or row it renames, and the Configure preview
    /// slip under its action, again whenever that moves.
    fn synchronize_slip(&mut self, root: &RenderRoot, context: &Context) {
        let inputs = match context.modal {
            Some(Modal::Rename) => false,
            Some(Modal::PreviewInputs) => true,
            _ => {
                self.slip = None;
                return;
            }
        };
        if root.needs_rewrite_passes() {
            return;
        }
        let Some(corner) = slip_anchor(root) else {
            return;
        };
        // The inputs slip floats in the transcript under the head, so only its offset matters.
        let corner = if inputs {
            let Some(edge) = slip_host(root) else {
                return;
            };
            (corner.0.saturating_sub(edge), 0)
        } else {
            corner
        };
        if self.slip != Some(corner) {
            self.slip = Some(corner);
            let _ = context.proxy.message(Invocation {
                action: if inputs {
                    Action::PlaceInputsSlip { x: corner.0 }
                } else {
                    Action::PlaceRenameSlip {
                        x: corner.0,
                        y: corner.1,
                    }
                },
                generation: context.generation,
            });
        }
    }
    fn enter_modal(&mut self, root: &mut RenderRoot) {
        let Some(target) = default_focus(root) else {
            return;
        };
        self.modal_return = Some(root.focused_widget().filter(|focused| *focused != target));
        root.focus_on(Some(target));
    }
    /// Move focus to the first problem's wedge when the writer asks, once it has mounted.
    fn synchronize_problem(&mut self, root: &mut RenderRoot, context: &Context) {
        let request = context.attention.problem_request();
        if request != self.problem {
            self.problem = request;
            self.problem_pending = true;
        }
        if !self.problem_pending || root.needs_rewrite_passes() {
            return;
        }
        if let Some(wedge) = first_wedge(root) {
            self.problem_pending = false;
            root.focus_on(Some(wedge));
            root.handle_access_event(accesskit::ActionRequest {
                action: accesskit::Action::ScrollIntoView,
                target_tree: accesskit::TreeId::ROOT,
                target_node: wedge.to_raw().into(),
                data: None,
            });
        }
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot, context: &Context) {
        self.synchronize_modal(root, context);
        self.synchronize_slip(root, context);
        self.synchronize_problem(root, context);
        let picker = context.picker_open;
        let generation = context.generation;
        if picker != self.picker {
            if picker {
                self.return_to = root.focused_widget().or(self.last);
                place_picker(root, context, root.focused_widget());
                root.focus_on(find(root, Kind::Query));
            } else if !root.focus_on(self.return_to) || self.return_to.is_none() {
                let admitted = context
                    .editing
                    .as_ref()
                    .and_then(|target| super::manuscript::find(root.get_layer_root(0), target));
                root.focus_on(admitted);
            }
            self.picker = picker;
        } else if picker && self.picker_instance != context.picker_instance {
            root.focus_on(find(root, Kind::Query));
        }
        self.picker_instance = context.picker_instance.clone();
        if !picker && context.modal.is_none() {
            self.recover(root, context);
        }
        if !picker {
            self.last = root.focused_widget().or(self.last);
            self.revealed = None;
        } else if self.revealed != Some(generation) {
            if let Some(id) = find(root, Kind::ActiveResult) {
                root.handle_access_event(accesskit::ActionRequest {
                    action: accesskit::Action::ScrollIntoView,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: id.to_raw().into(),
                    data: None,
                });
            }
            self.revealed = Some(generation);
        }
    }
    /// Focus never falls out of the window: when the control holding it goes away, as a closed
    /// decision's invoker, a finished preview step's Continue or the line under a view switch, it
    /// moves to the marked successor, the line being written, or the marked fallback such as the
    /// chosen view segment. A window that has never held focus starts on its default control, such
    /// as the welcome page's chosen tab.
    fn recover(&mut self, root: &mut RenderRoot, context: &Context) {
        let focused = root.focused_widget();
        if focused != self.placed {
            self.placed = None;
        }
        // A control this host placed focus on that has lost its mark, as the segment of a view
        // the writer then switched away from, gives focus on as if it had gone.
        let stale = focused.is_some_and(|id| self.placed == Some(id) && !marked(root, id));
        if focused.is_none() && !self.held && self.last.is_none() {
            // Nothing has ever held focus here.
            let start = default_focus(root);
            if start.is_some() {
                self.placed = start;
                root.focus_on(start);
            }
        } else if stale || (focused.is_none() && self.held) {
            let target = return_focus(root)
                .or_else(|| {
                    context
                        .editing
                        .as_ref()
                        .and_then(|target| super::manuscript::find(root.get_layer_root(0), target))
                })
                .or_else(|| fallback_focus(root))
                .or_else(|| default_focus(root));
            if target.is_some() {
                self.placed = target;
                root.focus_on(target);
            }
        }
    }
    /// Note whether focus is held once every part of the host has had its pass.
    pub fn settle(&mut self, root: &RenderRoot) {
        self.held = root.focused_widget().is_some();
    }
    pub fn kind(&mut self, root: &RenderRoot, generation: CommandGeneration) -> Kind {
        let Some(focused) = root.focused_widget() else {
            self.field = None;
            return Kind::Other;
        };
        if let Some((cached, scope, kind)) = &self.field
            && *cached == focused
            && *scope == generation
        {
            return kind.clone();
        }
        // Picker selection can change a retained button's role in a new command scope.
        let mut widget = root.get_layer_root(0);
        let kind = loop {
            if widget.id() == focused {
                break role(widget);
            }
            if let Some(input) = widget.downcast::<TextInput>() {
                break if input.area_pod().id() == focused {
                    role(widget)
                } else {
                    Kind::Other
                };
            }
            let Some(child) = widget
                .children()
                .into_iter()
                .find(|child| child.ctx().has_focus_target())
            else {
                break Kind::Other;
            };
            widget = child;
        };
        self.field = Some((focused, generation, kind.clone()));
        kind
    }
}
fn role(widget: WidgetRef<'_, dyn Widget>) -> Kind {
    if widget
        .downcast::<Button>()
        .is_some_and(|row| row.get_prop::<picker::ActiveResult>().0)
    {
        return Kind::ActiveResult;
    }
    let Some(input) = widget.downcast::<TextInput>() else {
        return Kind::Other;
    };
    if let Some(target) = &input.get_prop::<TranslationField>().0 {
        Kind::Translation(target.clone())
    } else if input.get_prop::<crate::app::ProjectField>().0 {
        Kind::ProjectForm
    } else if input.get_prop::<picker::QueryField>().0 {
        Kind::Query
    } else if input.get_prop::<crate::view::Manuscript>().0
        || input.get_prop::<view::ArgumentField>().0
    {
        Kind::Manuscript
    } else {
        Kind::Text
    }
}
