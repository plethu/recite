//! Reading-position capture runs around every native event; it remembers its viewport.
use super::*;
use crate::tests::support;
use std::sync::Arc;
use xilem::masonry::{
    app::{RenderRootOptions, WindowSizePolicy},
    core::NewWidget,
    peniko::Blob,
    theme,
    widgets::{IndexedStack, Label},
};

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
