//! Reading-position capture runs around every native event; it remembers its viewport.
use super::*;
use crate::tests::support;
use std::sync::Arc;
use xilem::masonry::{
    app::{RenderRootOptions, WindowSizePolicy},
    core::NewWidget,
    peniko::Blob,
    theme,
    widgets::{IndexedStack, Label, TextInput},
};

fn root(widget: NewWidget<impl xilem::masonry::core::Widget>) -> RenderRoot {
    let mut root = RenderRoot::new(
        widget,
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(theme::default_property_set()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: (400, 150).into(),
            scale_factor: 1.,
            test_font: Some(Blob::new(Arc::new(
                include_bytes!(
                    "../../../../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"
                )
                .to_vec(),
            ))),
        },
    );
    root.redraw();
    root
}
fn passage(writer: &crate::Writer, index: usize) -> NewWidget<TextInput> {
    NewWidget::new(TextInput::new(&format!("Passage {index}"))).with_props(PassageField(Some(
        crate::attention::EditingTarget::passage(writer, format!("passage-{index}")),
    )))
}

/// A manuscript viewport beside a stashable sibling holding many unrelated widgets.
fn mounted(writer: &crate::Writer, active: usize) -> (RenderRoot, WidgetId) {
    let manuscript = NewWidget::new(Portal::new(NewWidget::new(
        Flex::column().with_fixed(NewWidget::new(Label::new("Manuscript"))),
    )))
    .with_props(ReadingViewport::capture(writer));
    let portal = manuscript.id();
    let map = (0..200).fold(Flex::column(), |column, index| {
        column.with_fixed(NewWidget::new(Label::new(format!("Map card {index}"))))
    });
    let stack = IndexedStack::new()
        .with(manuscript)
        .with(NewWidget::new(map))
        .with_active_child(active);
    let mut root = RenderRoot::new(
        NewWidget::new(stack),
        |_| {},
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
    root.redraw();
    (root, portal)
}
fn show(root: &mut RenderRoot, child: usize) {
    let stack = root.get_layer_root(0).id();
    root.edit_widget(stack, |mut widget| {
        IndexedStack::set_active_child(&mut widget.downcast(), child);
    });
    root.redraw();
}

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";

#[test]
fn capture_reuses_the_manuscript_viewport_while_it_is_shown_or_stashed() {
    let directory = support::project(SOURCE).unwrap();
    let writer = support::open(&directory).unwrap();
    let (mut root, portal) = mounted(&writer, 0);
    let mut position = Position::default();
    position.capture(&root);
    assert_eq!(
        position.previous.as_ref().map(|snapshot| snapshot.portal),
        Some(portal)
    );
    // A remembered viewport is revalidated in place instead of searching the window.
    assert_eq!(position.portal, Some(portal));
    show(&mut root, 1);
    position.capture(&root);
    assert!(
        position.previous.is_none(),
        "a stashed manuscript has no reading position"
    );
    assert_eq!(
        position.portal,
        Some(portal),
        "stashing keeps the remembered viewport"
    );
    show(&mut root, 0);
    position.capture(&root);
    assert_eq!(
        position.previous.as_ref().map(|snapshot| snapshot.portal),
        Some(portal)
    );
}

#[test]
fn a_workspace_opened_with_the_manuscript_stashed_finds_it_once() {
    let directory = support::project(SOURCE).unwrap();
    let writer = support::open(&directory).unwrap();
    let (mut root, portal) = mounted(&writer, 1);
    let mut position = Position::default();
    position.capture(&root);
    assert!(position.previous.is_none());
    assert_eq!(
        position.portal,
        Some(portal),
        "one search finds the stashed viewport"
    );
    show(&mut root, 0);
    position.capture(&root);
    assert_eq!(
        position.previous.as_ref().map(|snapshot| snapshot.portal),
        Some(portal)
    );
}

#[test]
fn scrolling_reuses_content_rows_and_a_content_change_measures_them_again() {
    let directory = support::project(SOURCE).unwrap();
    let writer = support::open(&directory).unwrap();
    let rows = (0..6).fold(Flex::column(), |column, index| {
        column.with_fixed(passage(&writer, index))
    });
    let content = NewWidget::new(rows);
    let flex = content.id();
    let manuscript =
        NewWidget::new(Portal::new(content)).with_props(ReadingViewport::capture(&writer));
    let portal = manuscript.id();
    let mut root = root(manuscript);
    let tops = |position: &Position| {
        position
            .previous
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|row| (row.id, row.bounds))
            .collect::<Vec<_>>()
    };
    let mut position = Position::default();
    position.capture(&root);
    let measured = tops(&position);
    assert_eq!(measured.len(), 6);
    root.edit_widget(portal, |mut widget| {
        Portal::set_viewport_pos(&mut widget.downcast::<Portal<Flex>>(), Point::new(0., 60.));
    });
    root.redraw();
    position.capture(&root);
    assert_eq!(
        tops(&position),
        measured,
        "scrolling keeps rows in content coordinates"
    );
    assert_eq!(position.previous.as_ref().unwrap().position.y, 60.);
    root.edit_widget(flex, |mut widget| {
        Flex::add_fixed(&mut widget.downcast::<Flex>(), passage(&writer, 6));
    });
    root.redraw();
    position.capture(&root);
    assert_eq!(
        tops(&position).len(),
        7,
        "a content change measures the rows again"
    );
}
