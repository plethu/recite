//! A real Xilem view loop with live workers, the Writer's own theme and fonts, and a rendered frame.
use super::record::Recorder;
use crate::native_host::keyboard::tab::past_panes;
use crate::tests::support::native_label;
use masonry::testing::{TestHarness, TestHarnessParams};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use xilem::core::{
    DynMessage, MessageCtx, ProxyError, RawProxy, SendMessage, ViewId, ViewPathTracker,
};
use xilem::masonry::{
    core::{DefaultProperties, PointerButton, Widget, WidgetId, WidgetRef},
    dpi::PhysicalSize,
    peniko::{Blob, Color},
    widgets::Passthrough,
};
use xilem::{ViewCtx, WidgetView};

/// The default properties and canvas colour a state's window would use right now.
pub(crate) struct Look {
    pub properties: Arc<DefaultProperties>,
    pub canvas: Color,
    /// A fresh copy of the properties for the window's first frame, when asked for.
    pub initial: Option<DefaultProperties>,
}

/// A logical window size and the device scale it is rasterised at.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}
impl Frame {
    pub const DEFAULT: Self = Self {
        width: 1200,
        height: 800,
        scale: 1.,
    };
    pub const MINIMUM: Self = Self {
        width: 900,
        height: 650,
        scale: 1.,
    };
    pub fn at(self, scale: f64) -> Self {
        Self { scale, ..self }
    }
    pub fn label(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

#[derive(Debug)]
struct Results(mpsc::Sender<(Arc<[ViewId]>, SendMessage)>);
impl RawProxy for Results {
    fn send_message(&self, path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.0
            .send((path, message))
            .map_err(|error| ProxyError::DriverFinished(error.0.1))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

/// Worker replies arrive in bursts; a quiet interval this long ends one settle.
const QUIET: Duration = Duration::from_millis(200);
/// Busy rounds before giving up; each round waits at most one quiet interval for workers.
const SETTLE_ROUNDS: usize = 150;

// Fields drop in order: queued worker receipts go before the runtime that waits for their workers.
pub(crate) struct Stage<S: 'static, V: WidgetView<S, Widget = Passthrough>> {
    responses: mpsc::Receiver<(Arc<[ViewId]>, SendMessage)>,
    pub state: S,
    pub harness: TestHarness<Passthrough>,
    context: ViewCtx,
    view: V,
    local: V::ViewState,
    make: fn(&mut S) -> V,
    look: fn(&mut S, bool) -> Look,
    properties: Arc<DefaultProperties>,
    pub frame: Frame,
    /// Problems that do not stop a capture but make a frame less trustworthy.
    pub warnings: Vec<String>,
}
impl<S: 'static, V: WidgetView<S, Widget = Passthrough>> Stage<S, V> {
    pub fn new(
        mut state: S,
        make: fn(&mut S) -> V,
        look: fn(&mut S, bool) -> Look,
        frame: Frame,
    ) -> Self {
        let (sender, responses) = mpsc::channel();
        let runtime = Arc::new(tokio::runtime::Runtime::new().expect("gallery runtime"));
        let mut context = ViewCtx::new(Arc::new(Results(sender)), runtime);
        let mut initial = look(&mut state, true);
        let view = make(&mut state);
        let (pod, local) = view.build(&mut context, &mut state);
        let mut params = TestHarnessParams::DEFAULT
            .with_size(PhysicalSize::new(
                (f64::from(frame.width) * frame.scale).round() as u32,
                (f64::from(frame.height) * frame.scale).round() as u32,
            ))
            .with_scale(frame.scale);
        params.background_color = initial.canvas;
        params.panic_on_rewrite_saturation = false;
        let defaults = initial.initial.take().unwrap_or_default();
        let mut harness = TestHarness::create_with(defaults, pod.new_widget, params);
        harness.set_default_properties(initial.properties.clone());
        for (_, bytes) in recite_writer_session::typography::BUNDLED_FONTS {
            harness
                .render_root()
                .register_fonts(Blob::new(Arc::new(bytes.to_vec())));
        }
        let mut stage = Self {
            state,
            harness,
            context,
            view,
            local,
            make,
            look,
            properties: initial.properties,
            responses,
            frame,
            warnings: vec![],
        };
        stage.settle();
        stage
    }

    /// Run actions, worker replies and rebuilds until nothing changes for a quiet interval.
    pub fn settle(&mut self) {
        for _ in 0..SETTLE_ROUNDS {
            let mut changed = false;
            while let Some((action, source)) = self.harness.pop_action_erased() {
                let Some(path) = self.context.get_id_path(source).cloned() else {
                    continue;
                };
                self.deliver(path.to_vec(), DynMessage(action));
                changed = true;
            }
            let wait = if changed { Duration::ZERO } else { QUIET };
            if let Ok((path, output)) = self.responses.recv_timeout(wait) {
                self.deliver(path.to_vec(), output.into());
                while let Ok((path, output)) = self.responses.try_recv() {
                    self.deliver(path.to_vec(), output.into());
                }
                changed = true;
            }
            self.rebuild();
            // Layout during the rebuild can report sizes that the view lays out from, such as
            // the welcome form's measured height; those need another round.
            while let Some((action, source)) = self.harness.pop_action_erased() {
                let Some(path) = self.context.get_id_path(source).cloned() else {
                    continue;
                };
                self.deliver(path.to_vec(), DynMessage(action));
                changed = true;
            }
            if !changed {
                return;
            }
        }
        self.warnings
            .push("did not settle; the frame may show pending work".into());
    }

    fn deliver(&mut self, path: Vec<ViewId>, message: DynMessage) {
        let mut message =
            MessageCtx::new(std::mem::take(self.context.environment()), path, message);
        let (view, local, state) = (&mut self.view, &mut self.local, &mut self.state);
        self.harness
            .edit_root_widget(|root| view.message(local, &mut message, root, state));
        *self.context.environment() = message.finish().0;
    }

    fn rebuild(&mut self) {
        let look = (self.look)(&mut self.state, false);
        if !Arc::ptr_eq(&look.properties, &self.properties) {
            self.harness.set_default_properties(look.properties.clone());
            self.properties = look.properties;
        }
        let next = (self.make)(&mut self.state);
        let (view, local, context, state) = (
            &self.view,
            &mut self.local,
            &mut self.context,
            &mut self.state,
        );
        self.harness
            .edit_root_widget(|root| next.rebuild(view, local, context, root, state));
        self.view = next;
        self.harness.redraw();
    }

    /// Change state directly through a named app operation, then let the view catch up.
    pub fn act(&mut self, operation: impl FnOnce(&mut S)) {
        operation(&mut self.state);
        self.settle();
    }

    /// The live, visible widget whose accessible name is `name`.
    pub fn find(&self, name: &str) -> Option<WidgetId> {
        self.find_where(|label| label == name)
    }
    fn find_containing(&self, text: &str) -> Option<WidgetId> {
        self.find_where(|label| label.contains(text))
    }
    fn find_where(&self, matches: impl Fn(&str) -> bool) -> Option<WidgetId> {
        let mut pending: Vec<WidgetRef<'_, dyn Widget>> = vec![self.harness.root_widget().as_dyn()];
        while let Some(widget) = pending.pop() {
            if widget.ctx().is_stashed() {
                continue;
            }
            if self.context.get_id_path(widget.id()).is_some()
                && native_label(&self.harness, widget.id()).is_some_and(|label| matches(&label))
            {
                return Some(widget.id());
            }
            pending.extend(widget.children());
        }
        None
    }
    fn require(&mut self, name: &str) -> Option<WidgetId> {
        let id = self.find(name);
        if id.is_none() {
            self.warnings
                .push(format!("no live control named {name:?}"));
        }
        id
    }

    /// Activate a control the way assistive technology would.
    pub fn activate(&mut self, name: &str) -> bool {
        let Some(id) = self.require(name) else {
            return false;
        };
        self.click(id);
        true
    }
    /// Activate the first live control whose accessible name contains `text`.
    pub fn activate_containing(&mut self, text: &str) -> bool {
        let Some(id) = self.find_containing(text) else {
            self.warnings
                .push(format!("no live control containing {text:?}"));
            return false;
        };
        self.click(id);
        true
    }
    fn click(&mut self, id: WidgetId) {
        self.harness
            .process_access_event(masonry::accesskit::ActionRequest {
                action: masonry::accesskit::Action::Click,
                target_tree: masonry::accesskit::TreeId::ROOT,
                target_node: id.to_raw().into(),
                data: None,
            });
        self.settle();
    }
    pub fn hover(&mut self, name: &str) -> bool {
        let Some(id) = self.require(name) else {
            return false;
        };
        self.harness.mouse_move_to(id);
        self.settle();
        true
    }
    /// Hold the primary button down over a control, showing its pressed state.
    pub fn press(&mut self, name: &str) -> bool {
        if !self.hover(name) {
            return false;
        }
        self.harness
            .mouse_button_press(Some(PointerButton::Primary));
        self.harness.redraw();
        true
    }
    pub fn release(&mut self) {
        self.harness
            .mouse_button_release(Some(PointerButton::Primary));
        self.settle();
    }
    pub fn focus(&mut self, name: &str) -> bool {
        let Some(id) = self.require(name) else {
            return false;
        };
        self.harness.focus_on(Some(id));
        self.settle();
        true
    }
    pub fn tab(&mut self, backwards: bool) {
        self.harness.press_tab_key(backwards);
        // The native host passes over unnamed scroll panes; the gallery walks Tab the same way.
        past_panes(self.harness.render_root(), backwards);
        self.settle();
    }
    pub fn park_pointer(&mut self) {
        self.harness.mouse_move((-10., -10.));
        self.harness.redraw();
    }
    pub fn paste(&mut self, name: &str, text: &str) -> bool {
        if !self.focus(name) {
            return false;
        }
        self.harness
            .process_text_event(masonry::core::TextEvent::ClipboardPaste(text.into()));
        self.settle();
        true
    }
    /// Advance animation time without new input, for frames between two steps.
    pub fn animate(&mut self, ms: u64) {
        self.harness.animate_ms(ms);
        self.settle();
    }

    pub fn shot(&mut self, record: &mut Recorder, caption: &str) {
        self.harness.redraw();
        let image = self.harness.render();
        let warnings = std::mem::take(&mut self.warnings);
        record.frame(caption, image, &self.harness, self.frame, warnings);
    }
}
