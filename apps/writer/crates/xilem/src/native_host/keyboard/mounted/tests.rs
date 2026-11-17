//! Exercise the host's focus boundary against a real mounted widget tree.
use super::*;
use crate::app::picker::{ActiveResult, QueryField};
use crate::{Writer, session::Session, workspaces::Screen};
use crate::{
    attention::{Pane, PaneMotion, Region},
    view::Manuscript,
};
use xilem::masonry::{
    app::{RenderRootOptions, RenderRootSignal, WindowSizePolicy},
    core::{NewWidget, WidgetTag, WindowEvent as RootEvent},
    layout::AsUnit,
    peniko::Blob,
    widgets::{Button, Flex, Label, Passthrough, SizedBox, TextInput},
};

const SCRIPT: WidgetTag<TextInput> = WidgetTag::named("mounted-script");
const NAVIGATION: WidgetTag<TextInput> = WidgetTag::named("mounted-navigation");
const QUERY: WidgetTag<TextInput> = WidgetTag::named("mounted-query");
const MAP: WidgetTag<Button> = WidgetTag::named("mounted-map");
const RESULT: WidgetTag<Button> = WidgetTag::named("mounted-result");
#[path = "composition/tests.rs"]
mod composition;
#[path = "input/tests.rs"]
mod input;
#[path = "manuscript/tests.rs"]
mod manuscript;
#[path = "preview/tests.rs"]
mod preview;
#[path = "project/tests.rs"]
mod project;
#[path = "recovery/tests.rs"]
mod recovery;
#[path = "tab/tests.rs"]
mod tab;

fn mounted() -> (RenderRoot, mpsc::Receiver<RenderRootSignal>) {
    let (context, _) = context();
    let navigation = NewWidget::new(TextInput::new("Scene names")).with_tag(NAVIGATION);
    let script = NewWidget::new(TextInput::new("Keep this manuscript"))
        .with_tag(SCRIPT)
        .with_props(Manuscript(true));
    let panes = Flex::row()
        .with_fixed(
            NewWidget::new(SizedBox::new(navigation).width(200.px()).height(100.px()))
                .with_props(Region(Some(Pane::Navigation))),
        )
        .with_fixed(
            NewWidget::new(
                SizedBox::new(NewWidget::new(Button::with_text("Map card")).with_tag(MAP))
                    .width(200.px())
                    .height(100.px()),
            )
            .with_props(Region(Some(Pane::Map))),
        )
        .with_fixed(
            NewWidget::new(SizedBox::new(script).width(200.px()).height(100.px()))
                .with_props(Region(Some(Pane::Script))),
        );
    let body = Flex::column()
        .with_fixed(NewWidget::new(panes))
        .with_fixed(
            NewWidget::new(TextInput::new("Find a beat"))
                .with_tag(QUERY)
                .with_props(QueryField(true)),
        )
        .with_fixed(
            NewWidget::new(Button::new(NewWidget::new(Label::new("Active result"))))
                .with_tag(RESULT)
                .with_props(ActiveResult(true)),
        );
    let root = NewWidget::new(Passthrough::new(NewWidget::new(body)))
        .with_tag(TAG)
        .with_props(KeyboardContext(Some(Arc::new(context))));
    let (sender, signals) = mpsc::channel();
    let mut root = RenderRoot::new(
        root,
        move |signal| {
            sender.send(signal).unwrap();
        },
        RenderRootOptions {
            default_properties: Arc::new(theme::default_property_set()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: (900, 500).into(),
            scale_factor: 1.,
            test_font: Some(Blob::new(Arc::new(
                include_bytes!(
                    "../../../../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"
                )
                .to_vec(),
            ))),
        },
    );
    root.handle_window_event(RootEvent::EnableAccessTree);
    root.redraw();
    (root, signals)
}
fn field(root: &RenderRoot, tag: WidgetTag<TextInput>) -> WidgetId {
    root.get_widget_with_tag(tag).unwrap().area_pod().id()
}

#[test]
fn document_menu_dismissal_returns_to_its_origin_and_an_action_keeps_destination_focus() {
    use crate::attention::DocumentMenu;
    let (mut root, _signals) = mounted();
    let panel = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    root.edit_widget(panel, |mut widget| {
        widget.insert_prop(Region(Some(Pane::DocumentActions)));
    });
    let (mut context, _) = context();
    let script = field(&root, SCRIPT);
    let destination = field(&root, QUERY);
    let action = root.get_widget_with_tag(MAP).unwrap().id();
    let mut menu = document_menu::Focus::default();
    root.focus_on(Some(script));
    menu.synchronize(&mut root, &context);
    context.document_menu = DocumentMenu::Open;
    menu.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(action));
    context.document_menu = DocumentMenu::Dismissed;
    menu.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(script));
    context.document_menu = DocumentMenu::Open;
    menu.synchronize(&mut root, &context);
    root.focus_on(Some(destination));
    context.document_menu = DocumentMenu::Closed;
    menu.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(destination));
}

#[test]
fn picker_focus_returns_to_a_live_origin_and_clears_an_unadmitted_fallback() {
    let (mut root, signals) = mounted();
    let (mut context, _) = context();
    let generation = context.generation;
    let mut focus = focus::Focus::default();
    let manuscript = field(&root, SCRIPT);
    let navigation = field(&root, NAVIGATION);
    let query = field(&root, QUERY);
    root.focus_on(Some(manuscript));
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert!(focus.kind(&root, generation) == focus::Kind::Manuscript);
    context.picker_open = true;
    focus.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(query));
    assert!(focus.kind(&root, generation) == focus::Kind::Query);
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(manuscript));
    root.focus_on(Some(navigation));
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert!(focus.kind(&root, generation) == focus::Kind::Text);
    context.picker_open = true;
    focus.synchronize(&mut root, &context);
    let origin = root.get_widget_with_tag(NAVIGATION).unwrap().id();
    root.edit_widget(origin, |mut widget| widget.ctx.set_disabled(true));
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), None);
    let selected = root.get_widget_with_tag(RESULT).unwrap().id();
    root.focus_on(Some(selected));
    assert!(focus.kind(&root, generation) == focus::Kind::ActiveResult);
    assert!(
        signals
            .try_iter()
            .any(|signal| matches!(signal, RenderRootSignal::RequestRedraw))
    );
}

#[test]
fn directional_and_cyclic_pane_focus_use_live_geometry_and_restore_writing_focus() {
    let (mut root, _signals) = mounted();
    let (mut context, _) = context();
    let mut attention = panes::Attention::default();
    let mut writer = Writer::from_session(Session::examples().unwrap(), None);
    let script = field(&root, SCRIPT);
    let navigation = field(&root, NAVIGATION);
    let map = root.get_widget_with_tag(MAP).unwrap().id();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(script));
    for (motion, expected) in [
        (PaneMotion::Left, map),
        (PaneMotion::Left, navigation),
        (PaneMotion::Right, map),
        (PaneMotion::Next, script),
        (PaneMotion::Next, navigation),
        (PaneMotion::Previous, script),
        (PaneMotion::Up, script),
        (PaneMotion::Down, script),
    ] {
        writer.focus_pane(motion);
        context.attention = writer.attention.clone();
        attention.synchronize(&mut root, &context);
        assert_eq!(root.focused_widget(), Some(expected), "{motion:?}");
    }
    root.focus_on(Some(map));
    attention.synchronize(&mut root, &context);
    writer.toggle_focus();
    context.attention = writer.attention.clone();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(script));
    writer.toggle_focus();
    context.attention = writer.attention.clone();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(map));
    root.edit_widget(map, |mut widget| widget.ctx.set_disabled(true));
    root.focus_on(Some(navigation));
    writer.focus_pane(PaneMotion::Right);
    context.attention = writer.attention.clone();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(script));
    context.screen = Some(Screen::Build);
    attention.synchronize(&mut root, &context);
    context.screen = Some(Screen::Write);
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(script));
}

#[test]
fn composition_and_window_blur_guard_native_commands_and_preserve_the_text_field() {
    let (mut root, _signals) = mounted();
    let (context, messages) = context();
    let script = field(&root, SCRIPT);
    let query = field(&root, QUERY);
    root.focus_on(Some(script));
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    keyboard.event(
        &mut root,
        &WindowEvent::Ime(Ime::Preedit("pré".into(), Some((0, 3)))),
    );
    assert!(!keyboard.dispatch(
        &context,
        Some(script),
        focus::Kind::Manuscript,
        stroke(Key::Character("s"))
    ));
    assert!(messages.try_recv().is_err());
    root.focus_on(Some(query));
    assert!(keyboard.dispatch(
        &context,
        Some(query),
        focus::Kind::Query,
        stroke(Key::Character("s"))
    ));
    assert_eq!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::Command(WriterCommand::Save)
    );
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("".into(), None)));
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Commit("pré".into())));
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Disabled));
    keyboard.event(
        &mut root,
        &WindowEvent::ModifiersChanged(ModifiersState::CONTROL.into()),
    );
    keyboard.event(&mut root, &WindowEvent::Focused(false));
    assert_eq!(keyboard.modifiers, ModifiersState::default());
    assert!(matches!(keyboard.composition, Composition::Idle));
    assert_eq!(root.focused_widget(), Some(query));
    assert_eq!(field(&root, SCRIPT), script);
}

#[test]
fn rename_beat_on_a_sidebar_row_or_map_card_renames_that_beat_there() {
    use crate::app::keyboard::RenamesBeat;
    use crate::workspaces::rename::inline::{Place, Target};
    use xilem::winit::keyboard::{Key as NativeKey, KeyCode, PhysicalKey as NativePhysical};
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let (context, messages) = context();
    input::scope(&mut root, context);
    keyboard.synchronize(&mut root);
    let card = root.get_widget_with_tag(MAP).unwrap().id();
    root.focus_on(Some(card));
    let f2 = NativeKey::Named(NamedKey::F2);
    let press = Keystroke {
        logical_key: &f2,
        physical_key: NativePhysical::Code(KeyCode::F2),
        state: ElementState::Pressed,
        repeat: false,
    };
    let release = Keystroke {
        state: ElementState::Released,
        ..press
    };
    let rename = |root: &RenderRoot, keyboard: &mut Keyboard| {
        assert!(keyboard.key_event(root, press, false));
        assert!(keyboard.key_event(root, release, false));
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action
    };
    // Elsewhere F2 is the command, which renames the selected beat where it shows.
    assert_eq!(
        rename(&root, &mut keyboard),
        Action::Command(WriterCommand::Rename)
    );
    let target = Target {
        beat: "later".into(),
        place: Place::Card,
    };
    root.edit_widget(card, |mut widget| {
        widget.insert_prop(RenamesBeat(Some(target.clone())));
    });
    assert_eq!(rename(&root, &mut keyboard), Action::RenameBeat(target));
}

#[test]
fn a_closing_modal_returns_focus_to_the_successor_of_a_control_it_replaced() {
    use crate::app::keyboard::{DefaultFocus, ReturnFocus};
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let card = root.get_widget_with_tag(MAP).unwrap().id();
    let result = root.get_widget_with_tag(RESULT).unwrap().id();
    let script = root.get_widget_with_tag(SCRIPT).unwrap().id();
    root.edit_widget(script, |mut widget| {
        widget.insert_prop(DefaultFocus(true));
    });
    root.edit_widget(result, |mut widget| {
        widget.insert_prop(ReturnFocus(true));
    });
    root.focus_on(Some(card));
    // A rename opens over the card: its field takes focus.
    let (mut renaming, _) = context();
    renaming.modal = Some(Modal::Rename);
    input::scope(&mut root, renaming);
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(field(&root, SCRIPT)));
    // The card it was typed over was rebuilt meanwhile; focus goes to the marked successor.
    root.edit_widget(card, |mut widget| widget.ctx.set_disabled(true));
    let (context, _) = context();
    input::scope(&mut root, context);
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(result));
}

#[test]
fn focus_left_behind_a_mounted_modal_surface_returns_to_its_default_control() {
    use crate::app::keyboard::{DefaultFocus, ModalSurface};
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let card = root.get_widget_with_tag(MAP).unwrap().id();
    // The card's box stands in for a slip: a modal surface whose default control is the card.
    let mut pending = vec![root.get_layer_root(0)];
    let mut surface = None;
    while let Some(widget) = pending.pop() {
        if widget.downcast::<SizedBox>().is_some()
            && widget.children().iter().any(|child| child.id() == card)
        {
            surface = Some(widget.id());
        }
        pending.extend(widget.children());
    }
    root.edit_widget(surface.unwrap(), |mut widget| {
        widget.insert_prop(ModalSurface(true));
    });
    root.edit_widget(card, |mut widget| {
        widget.insert_prop(DefaultFocus(true));
    });
    let (mut renaming, _) = context();
    renaming.modal = Some(Modal::Rename);
    input::scope(&mut root, renaming);
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(card));
    // Focus falls behind the surface, onto the manuscript a rename's slip hangs over.
    root.focus_on(Some(field(&root, SCRIPT)));
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(card));
}

#[test]
fn a_rename_that_starts_again_elsewhere_in_one_pass_focuses_its_new_field() {
    use crate::app::keyboard::DefaultFocus;
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let (card, result) = (
        root.get_widget_with_tag(MAP).unwrap().id(),
        root.get_widget_with_tag(RESULT).unwrap().id(),
    );
    root.edit_widget(card, |mut widget| {
        widget.insert_prop(DefaultFocus(true));
    });
    let (mut renaming, _) = context();
    renaming.modal = Some(Modal::Rename);
    input::scope(&mut root, renaming);
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(card));
    // The heading's rename ends, focus goes back to the manuscript, and a sidebar row's rename
    // starts before the next pass: its field is the default now.
    root.edit_widget(card, |mut widget| {
        widget.insert_prop(DefaultFocus(false));
    });
    root.edit_widget(result, |mut widget| {
        widget.insert_prop(DefaultFocus(true));
    });
    root.focus_on(Some(field(&root, SCRIPT)));
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(result));
    // Focus the writer moves within the rename stays where they put it.
    root.focus_on(Some(card));
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(card));
}
