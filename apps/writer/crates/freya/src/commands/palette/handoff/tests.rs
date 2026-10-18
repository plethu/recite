use super::*;
use crate::{
    commands::{Command, SearchMode},
    design::{ModalState, SubmitAction},
    editing::Pane,
};
use freya_testing::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy)]
enum Change {
    None,
    Reopen,
    Settings,
    View,
    Typing,
    OtherModal,
    Dismiss,
}
#[derive(Clone)]
struct Handle(Rc<RefCell<Option<Controls>>>);
#[derive(Clone)]
struct Controls {
    handoff: Handoff,
    writer: Writer,
    modal: State<usize>,
}
fn app() -> impl IntoElement {
    let mut writer = crate::tests::support::writer();
    let mut modal = use_state(|| 0usize);
    use_provide_context(move || ModalState(modal));
    let handoff = use_handoff(writer);
    let handle = use_consume::<Handle>();
    let id = use_a11y();
    let change = use_consume::<Change>();
    use_hook(move || {
        writer.layout.navigation.set(false);
        writer.command_search.open(SearchMode::GoTo);
        *handle.0.borrow_mut() = Some(Controls {
            handoff,
            writer,
            modal,
        });
    });
    rect()
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if event.key == Key::Named(NamedKey::F12) {
                writer.command_search.close();
                writer.settings_open.set(false);
                writer.pane.set(Pane::Script);
                if matches!(change, Change::OtherModal) {
                    *modal.write() -= 1;
                }
            }
        })
        .maybe_child(writer.command_search.mode.read().is_some().then(|| {
            handoff.dialog(Dialog {
                title: "Palette lifecycle".into(),
                content: label().text("Go To").into_element(),
                actions: rect().into_element(),
                primary: SubmitAction {
                    id,
                    caption: "Activate".into(),
                    enabled: true,
                    action: EventHandler::new(move |()| {
                        handoff.submit(Item::Command(Command::Find), writer);
                        assert!(*handoff.mounted.peek());
                        assert!(
                            !*writer.layout.navigation.peek(),
                            "dispatch must wait for unmount"
                        );
                        match change {
                            Change::None => {}
                            Change::Reopen => writer.command_search.open(SearchMode::Commands),
                            Change::Settings => writer.settings_open.set(true),
                            Change::View => writer.pane.set(Pane::Map),
                            Change::Typing => writer.buffers.prose.set("Newer typing.".into()),
                            Change::OtherModal => *modal.write() += 1,
                            Change::Dismiss => handoff.dismiss(writer),
                        }
                    }),
                },
                focus_order: vec![id],
                close: EventHandler::new(|()| {}),
                reduced_motion: true,
                dismissal_only: true,
            })
        }))
}

#[test]
fn palette_dispatch_waits_for_unmount_and_newer_intents_cancel_permanently() {
    for change in [
        Change::None,
        Change::Reopen,
        Change::Settings,
        Change::View,
        Change::Typing,
        Change::OtherModal,
        Change::Dismiss,
    ] {
        let handle = Handle(Rc::new(RefCell::new(None)));
        let context = handle.clone();
        let mut test = TestingRunner::new(
            app,
            (900., 650.).into(),
            move |root| {
                root.provide_root_context(move || context);
                root.provide_root_context(move || change);
            },
            1.,
        )
        .0;
        let controls = handle.0.borrow().as_ref().unwrap().clone();
        assert_eq!(*controls.modal.peek(), 1, "the actual dialog is registered");
        let area = test
            .find(|node, e| {
                Rect::try_downcast(e)
                    .filter(|r| r.accessibility.builder.label() == Some("Activate"))
                    .map(|_| node.layout().area)
            })
            .unwrap();
        test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
        test.poll_n(std::time::Duration::from_millis(1), 8);
        assert!(controls.handoff.pending.peek().is_none());
        assert_eq!(
            *controls.writer.layout.navigation.peek(),
            matches!(change, Change::None)
        );
        test.press_key(Key::Named(NamedKey::F12));
        test.poll_n(std::time::Duration::from_millis(1), 8);
        assert_eq!(*controls.modal.peek(), 0);
        assert_eq!(
            *controls.writer.layout.navigation.peek(),
            matches!(change, Change::None),
            "cancelled work must never fire when a newer dialog closes"
        );
    }
}
