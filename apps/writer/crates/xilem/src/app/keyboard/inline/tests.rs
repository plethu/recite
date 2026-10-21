use super::*;
use crate::app::view;
use crate::{
    attention::PassageField,
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use tokio::sync::mpsc;
use xilem::masonry::{
    core::{PointerButton, TextEvent, WidgetId},
    kurbo::{Point, Vec2},
    widgets::Passthrough,
};

const SOURCE: &str = ":: start default\n> a@11111111111111111111\n  Alpha.\n> b@22222222222222222222\n  Bravo.\n-> END\n";

fn page(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    keyed((), view::app(app))
}

fn dispatch_focus(mounted: &mut Mounted<App, impl WidgetView<App, Widget = Passthrough>>) {
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    let (host, context) = loop {
        let widget = pending.pop().expect("tagged keyboard host");
        if let Some(host) = widget.downcast::<Passthrough>()
            && let Some(context) = host.get_prop::<KeyboardContext>().0.clone()
        {
            break (host.id(), context);
        }
        pending.extend(widget.children());
    };
    let target = mounted
        .harness
        .focused_widget_id()
        .and_then(|id| PassageField::focused(mounted.harness.root_widget().as_dyn(), id));
    // The real host submits this action synchronously after Masonry establishes
    // focus. It travels through the actual recorded action source and View path.
    mounted.harness.edit_widget_with_id(host, |mut widget| {
        widget.ctx.submit_untyped_action(Box::new(Invocation {
            generation: context.generation,
            action: Action::FieldFocus(target),
        }));
    });
    mounted.settle();
}

fn start_point(
    mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>,
    id: WidgetId,
) -> Point {
    mounted
        .harness
        .get_widget_with_id(id)
        .ctx()
        .window_transform()
        * Point::new(1., 8.)
}

#[test]
fn the_original_first_click_places_the_caret_and_routes_clean_focus_before_typing() {
    let directory = support::project(SOURCE).unwrap();
    let mut mounted = Mounted::new(App::with_writer(support::open(&directory).unwrap()), page);
    let second = mounted.find("Edit passage 2");
    mounted.harness.mouse_move(start_point(&mounted, second));
    mounted
        .harness
        .mouse_button_press(Some(PointerButton::Primary));
    dispatch_focus(&mut mounted);
    assert_eq!(mounted.find("Manuscript"), second);
    assert_eq!(mounted.harness.pointer_capture_target_id(), Some(second));
    mounted
        .harness
        .mouse_button_release(Some(PointerButton::Primary));
    mounted
        .harness
        .process_text_event(TextEvent::ClipboardPaste("K".into()));
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft(),
        "KBravo."
    );
    assert_eq!(mounted.find("Manuscript"), second);
}

#[test]
fn admitting_a_field_does_not_move_it_during_the_original_drag_selection() {
    let directory = support::project(SOURCE).unwrap();
    let mut mounted = Mounted::new(App::with_writer(support::open(&directory).unwrap()), page);
    mounted.settle();
    let second = mounted.find("Edit passage 2");
    let start = start_point(&mounted, second);
    mounted.harness.mouse_move(start);
    mounted
        .harness
        .mouse_button_press(Some(PointerButton::Primary));
    dispatch_focus(&mut mounted);
    assert_eq!(
        start_point(&mounted, second),
        start,
        "focus metadata must reserve its reading space"
    );
    mounted.harness.mouse_move(start + Vec2::new(100., 0.));
    mounted
        .harness
        .mouse_button_release(Some(PointerButton::Primary));
    mounted
        .harness
        .process_text_event(TextEvent::ClipboardPaste("Selected.".into()));
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft(),
        "Selected."
    );
    assert_eq!(mounted.find("Manuscript"), second);
}

#[test]
fn each_inline_field_keeps_its_native_undo_history_across_an_applied_focus_switch() {
    let directory = support::project(SOURCE).unwrap();
    let mut mounted = Mounted::new(App::with_writer(support::open(&directory).unwrap()), page);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    let writer = mounted.state.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    mounted.replace_text("Manuscript", "Edited alpha.");
    let first = mounted.find("Manuscript");
    let second = mounted.find("Edit passage 2");
    mounted
        .harness
        .mouse_click_on(second, Some(PointerButton::Primary));
    dispatch_focus(&mut mounted);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut jobs).unwrap();
    mounted.settle();
    assert_eq!(mounted.find("Manuscript"), second);
    mounted.replace_text("Manuscript", "Edited bravo.");
    mounted.harness.focus_on(Some(first));
    dispatch_focus(&mut mounted);
    support::finish_navigation(mounted.state.writer.as_mut().unwrap(), &mut jobs).unwrap();
    mounted.settle();
    assert_eq!(mounted.find("Manuscript"), first);
    use xilem::masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers};
    mounted
        .harness
        .process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character("z".into()),
            code: Code::KeyZ,
            modifiers: if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            },
            ..Default::default()
        }));
    mounted.settle();
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft(),
        "Alpha."
    );
    assert_eq!(mounted.find("Manuscript"), first);
}
