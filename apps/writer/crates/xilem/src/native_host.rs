//! Single-window host forwarding; command chords are handled before native text input.
mod accessibility;
mod keyboard;
use keyboard::Keyboard;
use masonry_winit::{
    app::{AppDriver, EventLoop, MasonryState, MasonryUserEvent, NewWindow},
    winit::{
        application::ApplicationHandler,
        event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
        event_loop::ActiveEventLoop,
        window::WindowId,
    },
};
use xilem::masonry::app;
use xilem::masonry::core::DefaultProperties;
use xilem::winit::error;

pub(super) fn run(
    event_loop: EventLoop,
    windows: Vec<NewWindow>,
    driver: impl AppDriver + 'static,
    defaults: DefaultProperties,
) -> Result<(), error::EventLoopError> {
    let _ = app::try_init_tracing();
    let mut host = Host {
        masonry: MasonryState::new(event_loop.create_proxy(), windows, defaults),
        driver,
        keyboard: Keyboard::default(),
    };
    event_loop.run_app(&mut host)
}
struct Host<D> {
    masonry: MasonryState,
    driver: D,
    keyboard: Keyboard,
}
impl<D: AppDriver> Host<D> {
    fn synchronize_focus(&mut self, event_loop: &ActiveEventLoop) {
        if !self.masonry.is_suspended() {
            {
                let mut roots = self.masonry.roots();
                if let (Some(root), None) = (roots.next(), roots.next()) {
                    self.keyboard.synchronize(root);
                }
            }
            self.masonry.flush_signals(event_loop, &mut self.driver);
        }
    }
}
impl<D: AppDriver> ApplicationHandler<MasonryUserEvent> for Host<D> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry.handle_resumed(event_loop, &mut self.driver);
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if !self.masonry.is_suspended() {
            let mut roots = self.masonry.roots();
            if let (Some(root), None) = (roots.next(), roots.next()) {
                self.keyboard.blur(root);
            } else {
                self.keyboard.reset();
            }
        } else {
            self.keyboard.reset();
        }
        self.masonry.handle_suspended(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.synchronize_focus(event_loop);
        let consumed = if self.masonry.is_suspended() {
            false
        } else {
            let mut roots = self.masonry.roots();
            match (roots.next(), roots.next()) {
                (Some(root), None) => self.keyboard.event(root, &event),
                // Recite currently owns one native window. Never route another root by guessing.
                _ => false,
            }
        };
        if !consumed {
            self.masonry
                .handle_window_event(event_loop, id, event, &mut self.driver);
        }
        self.synchronize_focus(event_loop);
    }
    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        self.masonry
            .handle_device_event(event_loop, id, event, &mut self.driver);
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: MasonryUserEvent) {
        self.synchronize_focus(event_loop);
        if accessibility::accepts(&mut self.masonry, &event) {
            self.masonry
                .handle_user_event(event_loop, event, &mut self.driver);
        }
        self.synchronize_focus(event_loop);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry.handle_about_to_wait(event_loop);
        self.synchronize_focus(event_loop);
    }
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.masonry.handle_new_events(event_loop, cause);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry.handle_exiting(event_loop);
    }
    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.masonry.handle_memory_warning(event_loop);
    }
}
