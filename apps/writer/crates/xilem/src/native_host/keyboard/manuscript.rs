//! Synchronous native focus admission precedes the next key, paste or IME commit.
use super::*;
use crate::attention::{EditingTarget, FieldRestoration, PassageField};
use xilem::masonry::core::{Widget, WidgetRef};

pub(super) fn hold_preedit(root: &RenderRoot) -> Option<crate::attention::Preedit> {
    let (_, target) = focused(root)?;
    let context = root
        .get_widget_with_tag(TAG)?
        .get_prop::<KeyboardContext>()
        .0
        .clone()?;
    (context.writer_active && !context.picker_open && context.editing.as_ref() == Some(&target))
        .then(|| context.attention.begin_preedit())
}

#[derive(Default)]
pub(super) struct Fields {
    observed: Option<(WidgetId, EditingTarget)>,
}
impl Fields {
    pub fn synchronize(
        &mut self,
        root: &mut RenderRoot,
        context: &Context,
        composition: &Composition,
    ) {
        let active = context.writer_active
            && !context.picker_open
            && root.get_layer_root(0).ctx().is_window_focused();
        if active
            && let Composition::Preedit {
                focus: Some(owner), ..
            } = composition
            && let Some(target) = &context.editing
            && find(root.get_layer_root(0), target) == Some(*owner)
            && root.focused_widget() != Some(*owner)
        {
            super::panes::reveal(root, *owner);
        }
        let restoring = active && context.field_restoration.is_some();
        if restoring {
            match context.field_restoration.as_ref() {
                Some(FieldRestoration::Clear)
                    if context.editing.is_none() && focused(root).is_some() =>
                {
                    root.focus_on(None);
                }
                Some(FieldRestoration::Passage(target))
                    if context.editing.as_ref() == Some(target) =>
                {
                    if let Some(field) = find(root.get_layer_root(0), target)
                        && root.focused_widget() != Some(field)
                    {
                        super::panes::reveal(root, field);
                    }
                }
                _ => {}
            }
        }
        let current = active.then(|| focused(root)).flatten();
        if current == self.observed && !restoring {
            return;
        }
        self.publish(root, context, current);
    }
    pub fn clear(&mut self, root: &mut RenderRoot, context: &Context) {
        if self.observed.is_some() {
            self.publish(root, context, None);
        }
    }
    fn publish(
        &mut self,
        root: &mut RenderRoot,
        context: &Context,
        current: Option<(WidgetId, EditingTarget)>,
    ) {
        self.observed = current.clone();
        let invocation = Invocation {
            action: Action::FieldFocus(current.map(|(_, target)| target)),
            generation: context.generation,
        };
        root.edit_widget_with_tag(TAG, |mut widget| {
            widget.ctx.submit_untyped_action(Box::new(invocation));
        });
    }
}
pub(super) fn focused(root: &RenderRoot) -> Option<(WidgetId, EditingTarget)> {
    let focused = root.focused_widget()?;
    PassageField::focused(root.get_layer_root(0), focused).map(|target| (focused, target))
}
pub(super) fn find(root: WidgetRef<'_, dyn Widget>, target: &EditingTarget) -> Option<WidgetId> {
    let mut pending = vec![root];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if let Some(input) = widget.downcast::<TextInput>()
            && input.get_prop::<PassageField>().0.as_ref() == Some(target)
        {
            return Some(input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}
pub(super) fn blocks_input(root: &RenderRoot, event: &WindowEvent) -> bool {
    unadmitted(root) && matches!(event, WindowEvent::Ime(Ime::Preedit(..) | Ime::Commit(_)))
}
pub(super) fn unadmitted(root: &RenderRoot) -> bool {
    let Some((_, target)) = focused(root) else {
        return false;
    };
    let admitted = root
        .get_widget_with_tag(TAG)
        .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        .is_some_and(|context| {
            context.writer_active
                && !context.picker_open
                && context.editing.as_ref() == Some(&target)
        });
    !admitted
}
