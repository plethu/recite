//! Native focus moves through RenderRoot's safe API; mounted fields retain their identities.
use super::*;
use crate::app::keyboard::CommandGeneration;
use crate::app::picker;
use crate::localisation::{TranslationField, TranslationTarget};
use crate::rules::view;
use xilem::masonry::accesskit;
use xilem::masonry::core::{Widget, WidgetRef};
use xilem::masonry::widgets;

#[derive(Clone, Default, PartialEq)]
pub(super) enum Kind {
    #[default]
    Other,
    Manuscript,
    Translation(TranslationTarget),
    Query,
    ActiveResult,
    Text,
}
#[derive(Default)]
pub(super) struct Focus {
    picker: bool,
    return_to: Option<WidgetId>,
    last: Option<WidgetId>,
    field: Option<(WidgetId, CommandGeneration, Kind)>,
    revealed: Option<CommandGeneration>,
}
impl Focus {
    pub fn synchronize(
        &mut self,
        root: &mut RenderRoot,
        picker: bool,
        generation: CommandGeneration,
    ) {
        if picker != self.picker {
            if picker {
                self.return_to = root.focused_widget().or(self.last);
                root.focus_on(find(root, Kind::Query));
            } else if !root.focus_on(self.return_to) || self.return_to.is_none() {
                root.focus_on(find(root, Kind::Manuscript));
            }
            self.picker = picker;
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
        .downcast::<widgets::Button>()
        .is_some_and(|button| button.get_prop::<picker::ActiveResult>().0)
    {
        return Kind::ActiveResult;
    }
    let Some(input) = widget.downcast::<TextInput>() else {
        return Kind::Other;
    };
    if let Some(target) = &input.get_prop::<TranslationField>().0 {
        Kind::Translation(target.clone())
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
fn find(root: &RenderRoot, kind: Kind) -> Option<WidgetId> {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if role(widget) == kind {
            if kind == Kind::ActiveResult {
                return Some(widget.id());
            }
            return widget
                .downcast::<TextInput>()
                .map(|input| input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}
