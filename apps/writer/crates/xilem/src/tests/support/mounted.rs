//! A real view/message/rebuild loop for component behavior without a desktop event loop.
use super::*;
use masonry::{
    accesskit,
    core::{TextEvent, Widget, WidgetId, WidgetRef},
    widgets::{Button, Passthrough, TextInput},
};
use xilem::core::MessageResult;

pub(crate) struct Mounted<S: 'static, V: WidgetView<S, Widget = Passthrough>> {
    pub state: S,
    pub harness: TestHarness<Passthrough>,
    context: ViewCtx,
    view: V,
    local: V::ViewState,
    make: fn(&mut S) -> V,
}
impl<S: 'static, V: WidgetView<S, Widget = Passthrough>> Mounted<S, V> {
    pub fn new(mut state: S, make: fn(&mut S) -> V) -> Self {
        let mut context = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = make(&mut state);
        let (pod, local) = view.build(&mut context, &mut state);
        let harness = TestHarness::create_with_size(
            masonry::theme::default_property_set(),
            pod.new_widget,
            (1200, 900),
        );
        Self {
            state,
            harness,
            context,
            view,
            local,
            make,
        }
    }
    pub fn find(&self, name: &str) -> WidgetId {
        let mut pending = vec![self.harness.root_widget().as_dyn()];
        while let Some(widget) = pending.pop() {
            if widget.ctx().is_stashed() {
                continue;
            }
            if self.context.get_id_path(widget.id()).is_some()
                && (native_label(&self.harness, widget.id()).as_deref() == Some(name)
                    || (widget.downcast::<Button>().is_some()
                        && self.contains_caption(widget, name)))
            {
                return widget.id();
            }
            pending.extend(widget.children());
        }
        panic!("Missing mounted control {name}");
    }
    pub fn contains_text(&self, text: &str) -> bool {
        let tree = self.harness.access_tree();
        let mut pending = vec![tree.state().root()];
        while let Some(node) = pending.pop() {
            if node.label().is_some_and(|caption| caption.contains(text))
                || node.value().is_some_and(|caption| caption.contains(text))
            {
                return true;
            }
            pending.extend(node.children());
        }
        false
    }
    fn contains_caption(&self, widget: WidgetRef<'_, dyn Widget>, name: &str) -> bool {
        let mut children = widget.children();
        while let Some(child) = children.pop() {
            if native_label(&self.harness, child.id()).as_deref() == Some(name) {
                return true;
            }
            children.extend(child.children());
        }
        false
    }
    pub fn activate(&mut self, name: &str) {
        let id = self.find(name);
        self.harness.process_access_event(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id.to_raw().into(),
            data: None,
        });
        self.settle();
    }
    /// The selected control whose name satisfies `matches`, such as a list's active row.
    fn selected_matching(&self, matches: impl Fn(&str) -> bool) -> Option<WidgetId> {
        let mut pending = vec![self.harness.root_widget().as_dyn()];
        while let Some(widget) = pending.pop() {
            if widget.ctx().is_stashed() {
                continue;
            }
            if native_label(&self.harness, widget.id()).is_some_and(|name| matches(&name))
                && self
                    .harness
                    .access_node(widget.id())
                    .is_some_and(|node| node.is_selected() == Some(true))
            {
                return Some(widget.id());
            }
            pending.extend(widget.children());
        }
        None
    }
    /// Whether some control named `name` is the selected one in its list.
    pub fn is_selected(&self, name: &str) -> bool {
        self.selected_matching(|caption| caption == name).is_some()
    }
    /// Activate the selected control named `name`.
    pub fn activate_selected(&mut self, name: &str) {
        let id = self
            .selected_matching(|caption| caption == name)
            .unwrap_or_else(|| panic!("Missing selected control {name}"));
        self.click(id);
    }
    /// Activate the selected control whose name starts with `prefix`.
    pub fn activate_selected_prefix(&mut self, prefix: &str) {
        let id = self
            .selected_matching(|caption| caption.starts_with(prefix))
            .unwrap_or_else(|| panic!("Missing selected control {prefix}…"));
        self.click(id);
    }
    fn click(&mut self, id: WidgetId) {
        self.harness.process_access_event(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id.to_raw().into(),
            data: None,
        });
        self.settle();
    }
    /// Step a named spin button once, as assistive technology would.
    pub fn step(&mut self, name: &str, up: bool) {
        let id = self.find(name);
        self.harness.process_access_event(accesskit::ActionRequest {
            action: if up {
                accesskit::Action::Increment
            } else {
                accesskit::Action::Decrement
            },
            target_tree: accesskit::TreeId::ROOT,
            target_node: id.to_raw().into(),
            data: None,
        });
        self.settle();
    }
    pub fn activate_prefix(&mut self, prefix: &str) {
        let mut pending = vec![self.harness.root_widget().as_dyn()];
        let caption = loop {
            let widget = pending.pop().expect("mounted action caption");
            if widget.ctx().is_stashed() {
                continue;
            }
            if let Some(caption) = native_label(&self.harness, widget.id())
                .filter(|caption| caption.starts_with(prefix))
            {
                break caption;
            }
            pending.extend(widget.children());
        };
        self.activate(&caption);
    }
    pub fn type_text(&mut self, name: &str, text: &str) {
        self.focus_field(name);
        self.harness
            .process_text_event(TextEvent::ClipboardPaste(text.into()));
        self.settle();
    }
    pub fn replace_text(&mut self, name: &str, text: &str) {
        use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers};
        self.focus_field(name);
        self.harness
            .process_text_event(TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Character("a".into()),
                code: Code::KeyA,
                modifiers: if cfg!(target_os = "macos") {
                    Modifiers::META
                } else {
                    Modifiers::CONTROL
                },
                ..KeyboardEvent::default()
            }));
        self.harness
            .process_text_event(TextEvent::ClipboardPaste(text.into()));
        self.settle();
    }
    fn focus_field(&mut self, name: &str) {
        let id = self.find(name);
        let area = self
            .harness
            .get_widget_with_id(id)
            .downcast::<TextInput>()
            .map_or(id, |input| input.area_pod().id());
        self.harness.focus_on(Some(area));
    }
    pub fn settle(&mut self) {
        for _ in 0..20 {
            let mut changed = false;
            while let Some((action, source)) = self.harness.pop_action_erased() {
                let path = self
                    .context
                    .get_id_path(source)
                    .expect("live native action")
                    .clone();
                let mut message = MessageCtx::new(
                    std::mem::take(self.context.environment()),
                    path,
                    DynMessage(action),
                );
                let result = self.harness.edit_root_widget(|root| {
                    self.view
                        .message(&mut self.local, &mut message, root, &mut self.state)
                });
                assert!(
                    !matches!(result, MessageResult::Stale),
                    "live mounted control must route to its view"
                );
                *self.context.environment() = message.finish().0;
                changed = true;
            }
            let next = (self.make)(&mut self.state);
            self.harness.edit_root_widget(|root| {
                next.rebuild(
                    &self.view,
                    &mut self.local,
                    &mut self.context,
                    root,
                    &mut self.state,
                )
            });
            self.view = next;
            self.harness.redraw();
            if !changed {
                return;
            }
        }
        panic!("Mounted component did not settle");
    }
}
