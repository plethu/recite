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
