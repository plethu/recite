use super::*;
use xilem::masonry::{
    kurbo::{Point, Rect},
    widgets::Portal,
};

const LATE: WidgetTag<TextInput> = WidgetTag::named("late-manuscript");
const SEARCH: WidgetTag<TextInput> = WidgetTag::named("late-search");
const VIEWPORT: WidgetTag<Portal<Flex>> = WidgetTag::named("late-script-viewport");

fn late() -> (RenderRoot, mpsc::Receiver<RenderRootSignal>, EditingTarget) {
    let (mut root, signals, target, _) = fields();
    let field = NewWidget::new(TextInput::new("Late passage"))
        .with_tag(LATE)
        .with_props(PassageField(Some(target.clone())));
    let content = Flex::column()
        .with_fixed(NewWidget::new(SizedBox::empty().height(800.px())))
        .with_fixed(NewWidget::new(SizedBox::new(field).height(100.px())))
        .with_fixed(NewWidget::new(SizedBox::empty().height(400.px())));
    let body = Flex::column()
        .with_fixed(
            NewWidget::new(TextInput::new(""))
                .with_tag(SEARCH)
                .with_props(QueryField(true)),
        )
        .with_fixed(NewWidget::new(
            SizedBox::new(NewWidget::new(Portal::new(NewWidget::new(content))).with_tag(VIEWPORT))
                .width(600.px())
                .height(400.px()),
        ));
    let root_id = root.get_layer_root(0).id();
    root.edit_widget(root_id, |mut widget| {
        Passthrough::set_child(&mut widget.downcast(), NewWidget::new(body));
    });
    root.redraw();
    (root, signals, target)
}
fn reset(root: &mut RenderRoot) {
    let viewport = root.get_widget_with_tag(VIEWPORT).unwrap().id();
    root.edit_widget(viewport, |mut widget| {
        Portal::set_viewport_pos(&mut widget.downcast::<Portal<Flex>>(), Point::ORIGIN);
    });
    root.redraw();
}
fn bounds(root: &RenderRoot) -> (Rect, Rect) {
    let viewport = root.get_widget_with_tag(VIEWPORT).unwrap();
    let editor = root.get_widget_with_tag(LATE).unwrap();
    (
        viewport
            .ctx()
            .window_transform()
            .transform_rect_bbox(viewport.ctx().border_box()),
        editor
            .ctx()
            .window_transform()
            .transform_rect_bbox(editor.ctx().border_box()),
    )
}

#[test]
fn closing_search_reveals_the_whole_already_focused_editor_only_once() {
    let (mut root, _signals, target) = late();
    let mut focus = focus::Focus::default();
    let mut attention = crate::attention::Attention::default();
    let mut context = context().0;
    context.editing = Some(target.clone());
    let field = field(&root, LATE);
    root.focus_on(Some(field));
    context.picker_open = true;
    focus.synchronize(&mut root, &context);
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(field));
    reset(&mut root);
    let (viewport, editor) = bounds(&root);
    assert!(editor.y0 >= viewport.y1);
    attention.request_edit(target.clone());
    focus.synchronize_manuscript(&mut root, Some(&target), attention.edit_request());
    root.redraw();
    let (viewport, editor) = bounds(&root);
    assert!(
        viewport.contains_rect(editor),
        "editor {editor:?}, viewport {viewport:?}"
    );
    assert!((editor.center().y - viewport.center().y).abs() < 1.);
    reset(&mut root);
    focus.synchronize_manuscript(&mut root, Some(&target), attention.edit_request());
    root.redraw();
    assert_eq!(
        root.get_widget_with_tag(VIEWPORT)
            .unwrap()
            .get_viewport_pos(),
        Point::ORIGIN
    );
}

#[test]
fn explicit_reveal_waits_for_its_matching_field_to_mount() {
    let (mut root, _signals, target) = late();
    let mut focus = focus::Focus::default();
    let mut attention = crate::attention::Attention::default();
    attention.request_edit(target.clone());
    let control = root.get_widget_with_tag(LATE).unwrap().id();
    root.edit_widget(control, |mut widget| {
        widget.insert_prop(PassageField(None));
    });
    focus.synchronize_manuscript(&mut root, Some(&target), attention.edit_request());
    root.edit_widget(control, |mut widget| {
        widget.insert_prop(PassageField(Some(target.clone())));
    });
    focus.synchronize_manuscript(&mut root, Some(&target), attention.edit_request());
    assert_eq!(root.focused_widget(), Some(field(&root, LATE)));
}
