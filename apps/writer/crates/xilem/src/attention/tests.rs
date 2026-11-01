use super::*;
use crate::{keyed, presentation, tests::support, view};
use std::sync::Arc;
use tokio::sync::mpsc as async_mpsc;
use xilem::{
    ViewCtx, WidgetView,
    core::View as _,
    masonry::{
        core::{
            TextEvent, WidgetId,
            keyboard::{Key, KeyState, KeyboardEvent, Modifiers},
        },
        testing::TestHarness,
        widgets::{Passthrough, TextInput},
    },
};

fn surface(writer: &Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed::keyed((), view::writer(writer))
}
fn manuscript(harness: &TestHarness<Passthrough>) -> WidgetId {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if let Some(input) = widget.downcast::<TextInput>()
            && input.get_prop::<view::Manuscript>().0
        {
            return input.area_pod().id();
        }
        pending.extend(widget.children());
    }
    panic!("mounted manuscript");
}

#[test]
fn focused_examples_keep_the_discard_notice_visible() -> Result<(), Box<dyn std::error::Error>> {
    for scale in [100, 200] {
        let mut writer = Writer::from_session(crate::session::Session::examples()?, None);
        writer.config.writer.presentation = writer
            .config
            .writer
            .presentation
            .with_value(recite_config::WriterPresentationField::UiScale, scale)?;
        writer.toggle_focus();
        let mut context = ViewCtx::new(
            Arc::new(support::NoProxy),
            Arc::new(tokio::runtime::Runtime::new()?),
        );
        let (pod, _) = surface(&writer).build(&mut context, &mut writer);
        let mut harness = TestHarness::create_with_size(
            presentation::Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            (1280, 720),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut notice = false;
        while let Some(node) = pending.pop() {
            notice |= node.label().or_else(|| node.value()).is_some_and(|label| {
                label == "Temporary example · changes are discarded when this window closes"
            });
            pending.extend(node.children());
        }
        assert!(notice);
        if let Ok(prefix) = std::env::var("RECITE_CAPTURE_FOCUS") {
            harness
                .render()
                .save(format!("{prefix}-temporary-{scale}.png"))?;
        }
    }
    Ok(())
}

#[test]
fn focus_stashes_surrounding_panes_and_preserves_native_field_identity_and_undo()
-> Result<(), Box<dyn std::error::Error>> {
    for scale in [100, 200] {
        for workspace in [Workspace::Script, Workspace::Map, Workspace::Split] {
            focus_lifetime(scale, workspace)?;
        }
    }
    Ok(())
}
fn focus_lifetime(scale: u16, workspace: Workspace) -> Result<(), Box<dyn std::error::Error>> {
    let directory = support::project(":: start default\n> @ffee6p8\n  First.\n-> END\n")?;
    let mut writer = support::open(&directory)?;
    writer.config.writer.presentation = writer
        .config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, scale)?;
    let runtime = Arc::new(tokio::runtime::Runtime::new()?);
    let mut context = ViewCtx::new(Arc::new(support::NoProxy), runtime.clone());
    let mut view = surface(&writer);
    let (pod, mut state) = view.build(&mut context, &mut writer);
    let mut harness = TestHarness::create_with_size(
        presentation::Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 720),
    );
    let (sender, mut work) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    let field = manuscript(&harness);
    harness.focus_on(Some(field));
    harness.keyboard_type_chars("x");
    support::flush_native(
        &mut writer,
        &mut harness,
        &mut context,
        &mut view,
        &mut state,
        (&mut work, &mut navigation),
        surface,
    )?;
    let typed = writer.session.model().draft().to_owned();
    assert!(typed.contains('x'));
    writer.map.workspace = workspace;
    let underlying = workspace;
    for focused in [true, false, true] {
        writer.toggle_focus();
        support::flush_native(
            &mut writer,
            &mut harness,
            &mut context,
            &mut view,
            &mut state,
            (&mut work, &mut navigation),
            surface,
        )?;
        assert_eq!(writer.attention.is_focused(), focused);
        assert_eq!(writer.map.workspace, underlying);
        assert_eq!(manuscript(&harness), field);
        assert_eq!(writer.session.model().draft(), typed);
        if focused && let Ok(prefix) = std::env::var("RECITE_CAPTURE_FOCUS") {
            harness.render().save(format!("{prefix}-{scale}.png"))?;
        }
    }
    harness.focus_on(Some(field));
    harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
        state: KeyState::Down,
        key: Key::Character("z".into()),
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
        ..KeyboardEvent::default()
    }));
    support::flush_native(
        &mut writer,
        &mut harness,
        &mut context,
        &mut view,
        &mut state,
        (&mut work, &mut navigation),
        surface,
    )?;
    assert_eq!(writer.session.model().draft(), "First.");
    Ok(())
}
