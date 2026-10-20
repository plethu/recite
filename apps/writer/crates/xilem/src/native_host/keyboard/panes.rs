//! Native pane geometry and focus history stay outside product commands.
use super::*;
use crate::attention::{Pane, PaneMotion, PaneRequest, Region};
use crate::workspaces::Screen;
use xilem::masonry::{
    accesskit,
    core::{Widget, WidgetRef},
    kurbo::Point,
    widgets::{Flex, SizedBox, TextArea},
};

#[derive(Default)]
enum WritingFocus {
    #[default]
    Workspace,
    Focused {
        return_to: Option<WidgetId>,
    },
}
#[derive(Default)]
pub(super) struct Attention {
    writing: WritingFocus,
    request: Option<PaneRequest>,
    last: Option<WidgetId>,
    screen: Option<Screen>,
    writing_target: Option<WidgetId>,
}
struct Target {
    pane: Pane,
    center: Point,
    focused: bool,
    widget: WidgetId,
}
impl Attention {
    pub fn synchronize(&mut self, root: &mut RenderRoot, context: &Context) {
        if context.picker_open {
            return;
        }
        let Some(screen) = context.screen else {
            return;
        };
        if self.screen != Some(screen) {
            if screen == Screen::Write {
                if self
                    .writing_target
                    .is_none_or(|target| !reveal(root, target))
                {
                    focus_script(root);
                }
            } else {
                let pane = match screen {
                    Screen::Declarations => Pane::Declarations,
                    Screen::Build => Pane::Build,
                    Screen::Settings => Pane::Settings,
                    Screen::Write => Pane::Script,
                };
                if let Some(target) = targets(root).iter().find(|target| target.pane == pane) {
                    reveal(root, target.widget);
                }
            }
            self.screen = Some(screen);
        }
        if screen != Screen::Write {
            return;
        }
        self.writing_target = root.focused_widget().or(self.writing_target);
        if matches!(self.writing, WritingFocus::Workspace) && !context.attention.is_focused() {
            self.last = root.focused_widget().or(self.last);
        }
        match (&self.writing, context.attention.is_focused()) {
            (WritingFocus::Workspace, true) => {
                self.writing = WritingFocus::Focused {
                    return_to: root.focused_widget().or(self.last),
                };
                focus_script(root);
            }
            (WritingFocus::Focused { return_to }, false) => {
                if return_to.is_none_or(|id| !reveal(root, id)) {
                    focus_script(root);
                }
                self.writing = WritingFocus::Workspace;
            }
            _ => {}
        }
        if self.request.as_ref() != context.attention.pane_request() {
            self.request = context.attention.pane_request().cloned();
            if let Some(request) = &self.request {
                move_focus(root, request.motion());
            }
        }
    }
}
fn focus_script(root: &mut RenderRoot) {
    let panes = targets(root);
    if let Some(target) = panes.iter().find(|target| target.pane == Pane::Script) {
        reveal(root, target.widget);
    }
}
fn move_focus(root: &mut RenderRoot, direction: PaneMotion) {
    let panes = targets(root);
    let Some(origin) = panes
        .iter()
        .find(|target| target.focused)
        .or_else(|| panes.iter().find(|target| target.pane == Pane::Script))
    else {
        return;
    };
    let (dx, dy) = match direction {
        PaneMotion::Left => (-1., 0.),
        PaneMotion::Right => (1., 0.),
        PaneMotion::Up => (0., -1.),
        PaneMotion::Down => (0., 1.),
        PaneMotion::Next | PaneMotion::Previous => {
            let index = panes
                .iter()
                .position(|target| target.pane == origin.pane)
                .unwrap_or(0);
            let step = if direction == PaneMotion::Next {
                1
            } else {
                panes.len() - 1
            };
            reveal(root, panes[(index + step) % panes.len()].widget);
            return;
        }
    };
    let target = panes
        .iter()
        .filter(|target| target.pane != origin.pane)
        .filter_map(|target| {
            let offset = target.center - origin.center;
            let forward = offset.x * dx + offset.y * dy;
            let sideways = (offset.x * dy - offset.y * dx).abs();
            (forward > 1. && forward > sideways).then_some((target.widget, offset.hypot()))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1));
    if let Some((id, _)) = target {
        reveal(root, id);
    }
}
fn reveal(root: &mut RenderRoot, id: WidgetId) -> bool {
    if root.focus_on(Some(id)) {
        root.handle_access_event(accesskit::ActionRequest {
            action: accesskit::Action::ScrollIntoView,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id.to_raw().into(),
            data: None,
        });
        return true;
    }
    false
}
fn targets(root: &RenderRoot) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if let Some(pane) = region(widget) {
            if let Some(target) = first_focusable(widget) {
                targets.push(Target {
                    pane,
                    center: widget.ctx().window_transform() * widget.ctx().border_box().center(),
                    focused: widget.ctx().has_focus_target(),
                    widget: target,
                });
            }
        } else {
            pending.extend(widget.children());
        }
    }
    targets.sort_by_key(|target| target.pane);
    targets
}
fn region(widget: WidgetRef<'_, dyn Widget>) -> Option<Pane> {
    if let Some(widget) = widget.downcast::<SizedBox>() {
        widget.get_prop::<Region>().0
    } else {
        widget
            .downcast::<Flex>()
            .and_then(|widget| widget.get_prop::<Region>().0)
    }
}
fn first_focusable(widget: WidgetRef<'_, dyn Widget>) -> Option<WidgetId> {
    let mut pending = vec![widget];
    let mut fallback = None;
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if widget.ctx().accepts_focus() {
            // Prefer a writing field over the pane's surrounding action buttons.
            if widget.downcast::<TextArea<true>>().is_some() {
                return Some(widget.id());
            }
            fallback.get_or_insert(widget.id());
        }
        pending.extend(widget.children().into_iter().rev());
    }
    fallback
}
