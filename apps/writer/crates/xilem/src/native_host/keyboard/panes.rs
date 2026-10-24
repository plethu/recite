//! Native pane geometry and focus history stay outside product commands.
use super::*;
use crate::attention::{Pane, PaneEntry, PaneMotion, PaneRequest, Region};
use crate::workspaces::Screen;
use xilem::masonry::{
    accesskit,
    core::{Widget, WidgetRef},
    kurbo::Point,
    widgets::{Flex, Portal, SizedBox, TextArea},
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
                    Screen::Rename => Pane::Rename,
                    Screen::External => Pane::External,
                    Screen::Localisation => Pane::Localisation,
                    Screen::Write => Pane::Script,
                };
                if let Some(target) = targets(root).iter().find(|target| target.pane == pane) {
                    reveal(root, target.widget);
                }
            }
            self.screen = Some(screen);
        }
        if screen != Screen::Write {
            self.synchronize_request(root, context);
            return;
        }
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
        self.synchronize_request(root, context);
        self.writing_target = root.focused_widget().or(self.writing_target);
    }
    fn synchronize_request(&mut self, root: &mut RenderRoot, context: &Context) {
        if self.request.as_ref() != context.attention.pane_request() {
            self.request = context.attention.pane_request().cloned();
            if let Some(request) = &self.request {
                move_focus(root, request.motion());
            }
        }
    }
}
pub(super) fn focus_script(root: &mut RenderRoot) {
    let panes = targets(root);
    if let Some(target) = panes.iter().find(|target| target.pane == Pane::Script) {
        reveal(root, target.widget);
    }
}
pub(super) fn focus_pane(root: &mut RenderRoot, pane: Pane) -> bool {
    targets(root)
        .iter()
        .find(|target| target.pane == pane)
        .is_some_and(|target| reveal(root, target.widget))
}
fn move_focus(root: &mut RenderRoot, direction: PaneMotion) {
    let panes = targets(root);
    if let PaneMotion::To(pane) = direction {
        if let Some(target) = panes.iter().find(|target| target.pane == pane) {
            reveal(root, target.widget);
        }
        return;
    }
    let Some(origin) = panes
        .iter()
        .find(|target| target.focused)
        .or_else(|| panes.iter().find(|target| target.pane == Pane::Script))
    else {
        return;
    };
    let (dx, dy) = match direction {
        PaneMotion::To(_) => return,
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
pub(super) fn reveal(root: &mut RenderRoot, id: WidgetId) -> bool {
    if root.focus_on(Some(id)) {
        reveal_focused(root);
        return true;
    }
    false
}
/// Explicit passage navigation gives the editor context without changing ordinary tabbing.
pub(super) fn reveal_editor(root: &mut RenderRoot, id: WidgetId) {
    if !reveal(root, id) {
        return;
    }
    let Some(focused) = root.focused_widget() else {
        return;
    };
    let control = focused_control(root, focused);
    let mut widget = root.get_layer_root(0);
    let mut viewport = None;
    while widget.id() != control {
        if widget.downcast::<Portal<Flex>>().is_some() {
            viewport = Some(widget.id());
        }
        let Some(child) = widget
            .children()
            .into_iter()
            .find(|child| child.ctx().has_focus_target())
        else {
            return;
        };
        widget = child;
    }
    let bounds = widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box());
    let Some(viewport) = viewport else {
        return;
    };
    let Some(portal) = root
        .get_widget(viewport)
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
    else {
        return;
    };
    let height = portal.ctx().content_box().height();
    let oversized = bounds.height() >= height;
    let anchor = if oversized {
        bounds.origin()
    } else {
        bounds.center()
    };
    let local = portal.ctx().to_local(anchor);
    let position = portal.get_viewport_pos();
    let top = local.y - if oversized { 0. } else { height / 2. };
    let position = Point::new(position.x, position.y + top);
    root.edit_widget(viewport, |mut widget| {
        Portal::set_viewport_pos(&mut widget.downcast::<Portal<Flex>>(), position);
    });
}
pub(super) fn reveal_focused(root: &mut RenderRoot) {
    if let Some(id) = root.focused_widget() {
        let control = focused_control(root, id);
        root.handle_access_event(accesskit::ActionRequest {
            action: accesskit::Action::ScrollIntoView,
            target_tree: accesskit::TreeId::ROOT,
            target_node: control.to_raw().into(),
            data: None,
        });
    }
}
fn focused_control(root: &RenderRoot, focused: WidgetId) -> WidgetId {
    let mut widget = root.get_layer_root(0);
    loop {
        if let Some(input) = widget.downcast::<TextInput>() {
            return if input.area_pod().id() == focused {
                input.id()
            } else {
                focused
            };
        }
        let Some(child) = widget
            .children()
            .into_iter()
            .find(|child| child.ctx().has_focus_target())
        else {
            return focused;
        };
        widget = child;
    }
}
fn targets(root: &RenderRoot) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if let Some(pane) = region(widget) {
            if let Some(target) = pane_entry(widget, pane) {
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
fn pane_entry(widget: WidgetRef<'_, dyn Widget>, pane: Pane) -> Option<WidgetId> {
    let mut pending = vec![widget];
    while let Some(child) = pending.pop() {
        if child.ctx().is_stashed() || child.ctx().is_disabled() {
            continue;
        }
        if child
            .downcast::<Flex>()
            .is_some_and(|flex| flex.get_prop::<PaneEntry>().0)
            && let Some(target) = first_focusable(child, false)
        {
            return Some(target);
        }
        pending.extend(child.children());
    }
    first_focusable(widget, pane != Pane::Preview)
}
fn first_focusable(widget: WidgetRef<'_, dyn Widget>, prefer_text: bool) -> Option<WidgetId> {
    let mut pending = vec![widget];
    let mut fallback = None;
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if widget.ctx().accepts_focus() {
            // Prefer a writing field over the pane's surrounding action buttons.
            if prefer_text && widget.downcast::<TextArea<true>>().is_some() {
                return Some(widget.id());
            }
            fallback.get_or_insert(widget.id());
        }
        pending.extend(widget.children().into_iter().rev());
    }
    fallback
}
