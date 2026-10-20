use super::*;
use std::sync::{Arc, mpsc};
use xilem::core::{MessageProxy, ProxyError, RawProxy, SendMessage, ViewId};

#[derive(Debug)]
struct Recorder(mpsc::Sender<SendMessage>);
impl RawProxy for Recorder {
    fn send_message(&self, _path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.0
            .send(message)
            .map_err(|error| ProxyError::DriverFinished(error.0))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}
fn context() -> (Context, mpsc::Receiver<SendMessage>) {
    let (sender, receiver) = mpsc::channel();
    (
        Context {
            shortcuts: Default::default(),
            generation: Default::default(),
            writer_active: true,
            picker_open: false,
            proxy: MessageProxy::new(Arc::new(Recorder(sender)), Arc::from([])),
        },
        receiver,
    )
}
fn stroke(key: Key<'_>) -> Stroke<'_> {
    Stroke {
        input: Input {
            key,
            physical: PhysicalKey::Other,
            modifiers: Modifiers {
                primary: true,
                ..Modifiers::default()
            },
        },
        enter: false,
        picker: None,
        first_press: true,
    }
}

#[test]
fn saved_bindings_take_effect_and_repeat_release_and_native_undo_do_not_post_actions() {
    let (mut context, receiver) = context();
    let mut keyboard = Keyboard::default();
    assert!(keyboard.dispatch(&context, None, false, stroke(Key::Character("s"))));
    let action = receiver
        .try_recv()
        .unwrap()
        .downcast::<Invocation>()
        .unwrap();
    assert_eq!(action.command, WriterCommand::Save);
    assert_eq!(action.generation, context.generation);
    let mut repeat = stroke(Key::Character("s"));
    repeat.first_press = false;
    assert!(keyboard.dispatch(&context, None, false, repeat));
    assert!(receiver.try_recv().is_err());
    context
        .shortcuts
        .rebind(
            WriterCommand::Save,
            "Primary+Alt+S".to_owned().try_into().unwrap(),
        )
        .unwrap();
    assert!(!keyboard.dispatch(&context, None, false, stroke(Key::Character("s"))));
    let mut rebound = stroke(Key::Character("s"));
    rebound.input.modifiers.alt = true;
    assert!(keyboard.dispatch(&context, None, false, rebound));
    assert_eq!(
        receiver
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .command,
        WriterCommand::Save
    );
    for character in ["z", "y", "c", "v", "x", "a"] {
        assert!(!keyboard.dispatch(&context, None, false, stroke(Key::Character(character))));
    }
    assert!(receiver.try_recv().is_err());
}

#[test]
fn apply_belongs_to_the_active_manuscript_and_composition_bypasses_commands() {
    let (mut context, receiver) = context();
    let mut keyboard = Keyboard::default();
    let harness = xilem::masonry::testing::TestHarness::create(
        xilem::masonry::theme::default_property_set(),
        xilem::masonry::core::NewWidget::new(xilem::masonry::widgets::Label::new(
            "Composition owner",
        )),
    );
    let focus = Some(harness.root_id());
    keyboard.composition = Composition::Preedit(focus);
    assert!(!keyboard.dispatch(&context, focus, true, stroke(Key::Character("s"))));
    assert!(receiver.try_recv().is_err());
    // Moving focus retires the old field's preedit guard.
    assert!(keyboard.dispatch(&context, None, false, stroke(Key::Character("s"))));
    receiver.try_recv().unwrap();
    for (active, manuscript, expected) in [
        (true, false, false),
        (false, true, false),
        (true, true, true),
    ] {
        context.writer_active = active;
        let mut enter = stroke(Key::Other);
        enter.enter = true;
        assert_eq!(
            keyboard.dispatch(&context, focus, manuscript, enter),
            expected
        );
        if expected {
            assert_eq!(
                receiver
                    .try_recv()
                    .unwrap()
                    .downcast::<Invocation>()
                    .unwrap()
                    .command,
                WriterCommand::Apply
            );
        } else {
            assert!(receiver.try_recv().is_err());
        }
    }
    keyboard.modifiers = ModifiersState::CONTROL;
    keyboard.composition = Composition::Preedit(focus);
    keyboard.reset();
    assert_eq!(keyboard.normalized_modifiers(), Modifiers::default());
    assert!(matches!(keyboard.composition, Composition::Idle));
}
