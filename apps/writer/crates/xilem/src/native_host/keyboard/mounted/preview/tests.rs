use super::*;
use crate::attention::PaneEntry;

const RETURN: WidgetTag<Button> = WidgetTag::named("preview-return");
const SETUP: WidgetTag<Button> = WidgetTag::named("preview-setup");

#[test]
fn preview_pane_prefers_setup_and_falls_back_to_return_when_setup_is_disabled() {
    let (mut root, _signals) = mounted();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    let body = Flex::column()
        .with_fixed(NewWidget::new(Button::with_text("Return to writing")).with_tag(RETURN))
        .with_fixed(
            NewWidget::new(
                Flex::column()
                    .with_fixed(NewWidget::new(Button::with_text("Start preview")).with_tag(SETUP)),
            )
            .with_props(PaneEntry(true)),
        );
    root.edit_widget(pane, |mut widget| {
        widget.insert_prop(Region(Some(Pane::Preview)));
        SizedBox::set_child(&mut widget.downcast::<SizedBox>(), NewWidget::new(body));
    });
    let setup = root.get_widget_with_tag(SETUP).unwrap().id();
    let returning = root.get_widget_with_tag(RETURN).unwrap().id();
    assert!(panes::focus_pane(&mut root, Pane::Preview));
    assert_eq!(root.focused_widget(), Some(setup));
    root.edit_widget(setup, |mut widget| widget.ctx.set_disabled(true));
    root.focus_on(None);
    assert!(panes::focus_pane(&mut root, Pane::Preview));
    assert_eq!(root.focused_widget(), Some(returning));
    assert!(!root.focus_on(Some(setup)));
}

#[test]
fn contextual_result_requests_reveal_once_without_overriding_later_focus() {
    let (mut root, _signals) = mounted();
    let (mut context, _) = context();
    let mut writer = Writer::from_session(Session::examples().unwrap(), None);
    let mut attention = panes::Attention::default();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    root.edit_widget(pane, |mut widget| {
        widget.insert_prop(Region(Some(Pane::External)));
    });
    let target = root.get_widget_with_tag(MAP).unwrap().id();
    let navigation = field(&root, NAVIGATION);
    context.screen = Some(Screen::External);
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(target));
    root.focus_on(Some(navigation));
    writer.focus_pane(PaneMotion::To(Pane::External));
    context.attention = writer.attention.clone();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(target));
    root.focus_on(Some(navigation));
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(navigation));
}
