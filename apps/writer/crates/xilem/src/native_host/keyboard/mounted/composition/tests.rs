//! A real host preedit lifetime guards the actual writing-view navigation callback.
use super::*;
use crate::native_host::keyboard::manuscript as native_fields;
use crate::{
    attention::EditingTarget,
    keyed::keyed,
    tests::support::{self, NoProxy},
};
use xilem::masonry::{
    accesskit,
    core::{Ime as RootIme, TextEvent},
    widgets::TextArea,
};
use xilem::{
    ViewCtx, WidgetView,
    core::{DynMessage, MessageCtx, MessageResult, View as _, ViewPathTracker},
};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn query_preedit_does_not_guard_manuscript_and_rejected_termination_releases_an_old_lease() {
    use crate::attention::PassageField;
    for ending in [
        Ime::Commit("é".into()),
        Ime::Preedit(String::new(), None),
        Ime::Disabled,
    ] {
        let (mut root, _signals) = mounted();
        let writer = Writer::from_session(Session::examples().unwrap(), None);
        let target = EditingTarget::capture(&writer).unwrap();
        let script = root.get_widget_with_tag(SCRIPT).unwrap().id();
        root.edit_widget(script, |mut widget| {
            widget.insert_prop(PassageField(Some(target.clone())));
        });
        let (mut initial, _) = context();
        initial.editing = Some(target);
        let attention = initial.attention.clone();
        let tag = root.get_widget_with_tag(TAG).unwrap().id();
        root.edit_widget(tag, |mut widget| {
            widget.insert_prop(KeyboardContext(Some(Arc::new(initial))));
        });
        root.focus_on(Some(field(&root, QUERY)));
        let mut keyboard = Keyboard::default();
        keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
        assert!(
            !attention.is_composing(),
            "query composition has its own native lifetime"
        );
        root.focus_on(Some(field(&root, SCRIPT)));
        keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
        assert!(attention.is_composing());
        let (mut refused, _) = context();
        refused.attention = attention.clone();
        root.edit_widget(tag, |mut widget| {
            widget.insert_prop(KeyboardContext(Some(Arc::new(refused))));
        });
        keyboard.event(&mut root, &WindowEvent::Ime(ending));
        assert!(
            !attention.is_composing(),
            "termination releases even when text delivery is refused"
        );
    }
}
#[test]
fn preedit_survives_the_real_next_button_and_commit_reaches_the_same_native_field() {
    let source = ":: start default\n".to_owned()
        + &(1..=40)
            .map(|n| format!("> line_{n}@{n:020x}\n  Line {n}.\n"))
            .collect::<String>()
        + "-> END\n";
    let directory = support::project(&source).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let original = writer.session.model().document().source_snapshot();
    let target = EditingTarget::capture(&writer).unwrap();
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = surface(&mut writer);
    let (pod, mut local) = view.build(&mut ctx, &mut writer);
    let (mut root, signals) = mounted();
    let (mut context, _) = context();
    context.editing = Some(target.clone());
    context.attention = writer.attention.clone();
    let outer = root.get_layer_root(0).id();
    root.edit_widget(outer, |mut widget| {
        widget.insert_prop(KeyboardContext(Some(Arc::new(context))));
        Passthrough::set_child(&mut widget.downcast(), pod.new_widget);
    });
    let update = root.redraw().1.unwrap();
    let mut button = update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Next beat entries"))
        .unwrap()
        .0;
    while update
        .nodes
        .iter()
        .find(|(id, _)| *id == button)
        .unwrap()
        .1
        .role()
        != accesskit::Role::Button
    {
        button = update
            .nodes
            .iter()
            .find(|(_, node)| node.children().contains(&button))
            .unwrap()
            .0;
    }
    let field = native_fields::find(root.get_layer_root(0), &target).unwrap();
    root.focus_on(Some(field));
    root.handle_text_event(TextEvent::WindowFocusChange(true));
    let mut keyboard = Keyboard::default();
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
    root.handle_text_event(TextEvent::Ime(RootIme::Preedit("é".into(), None)));
    assert!(writer.attention.is_composing());
    root.handle_access_event(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: button,
        data: None,
    });
    let child_root = root.get_layer_root(0).children()[0].id();
    for signal in signals.try_iter() {
        if let RenderRootSignal::Action(action, source) = signal {
            let path = ctx.get_id_path(source).unwrap().clone();
            let mut message =
                MessageCtx::new(std::mem::take(ctx.environment()), path, DynMessage(action));
            let result = root.edit_widget(child_root, |mut widget| {
                view.message(
                    &mut local,
                    &mut message,
                    widget.downcast::<Passthrough>(),
                    &mut writer,
                )
            });
            assert!(!matches!(result, MessageResult::Stale));
            *ctx.environment() = message.finish().0;
        }
    }
    let next = surface(&mut writer);
    root.edit_widget(child_root, |mut widget| {
        next.rebuild(
            &view,
            &mut local,
            &mut ctx,
            widget.downcast::<Passthrough>(),
            &mut writer,
        )
    });
    view = next;
    root.redraw();
    assert_eq!(writer.reading.window.start(), 0);
    assert!(!writer.navigation.busy());
    assert!(writer.status.contains("composition"));
    assert!(Arc::ptr_eq(
        &original,
        &writer.session.model().document().source_snapshot()
    ));
    assert_eq!(
        native_fields::find(root.get_layer_root(0), &target),
        Some(field)
    );
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Commit("é".into())));
    root.handle_text_event(TextEvent::Ime(RootIme::Commit("é".into())));
    assert!(!writer.attention.is_composing());
    let text = root
        .get_widget(field)
        .unwrap()
        .downcast::<TextArea<true>>()
        .unwrap()
        .text()
        .to_string();
    assert!(text.contains('é'));
    for signal in signals.try_iter() {
        if let RenderRootSignal::Action(action, source) = signal {
            let path = ctx.get_id_path(source).unwrap().clone();
            let mut message =
                MessageCtx::new(std::mem::take(ctx.environment()), path, DynMessage(action));
            root.edit_widget(child_root, |mut widget| {
                view.message(
                    &mut local,
                    &mut message,
                    widget.downcast::<Passthrough>(),
                    &mut writer,
                )
            });
            *ctx.environment() = message.finish().0;
        }
    }
    assert_eq!(writer.session.model().draft(), text);
}
