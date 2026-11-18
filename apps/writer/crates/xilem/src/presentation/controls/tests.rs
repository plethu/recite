use super::{super::Type, super::Typeset, prose};
use crate::tests::support::NoProxy;
use recite_writer_session::typography::{BUNDLED_FONTS, FONT_UI};
use std::sync::Arc;
use xilem::masonry::{
    accesskit::{Node, NodeId, Role, TreeUpdate},
    app::{RenderRoot, RenderRootOptions, WindowSizePolicy},
    core::{NewWidget, WindowEvent},
    peniko::Blob,
    theme::default_property_set,
    widgets::Flex,
};
use xilem::{ViewCtx, WidgetView, view};

/// The families of every text run `content` sets, with the bundled fonts and no system fonts.
fn families(content: impl WidgetView<(), ()>) -> Vec<String> {
    let mut context = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let (pod, _) = content.build(&mut context, &mut ());
    let root = NewWidget::new(Flex::column().with_fixed(pod.new_widget)).erased();
    let mut root = RenderRoot::new(
        root,
        |_| {},
        RenderRootOptions {
            default_properties: Arc::new(default_property_set()),
            use_system_fonts: false,
            size_policy: WindowSizePolicy::User,
            size: (400, 200).into(),
            scale_factor: 1.,
            test_font: None,
        },
    );
    for (_, bytes) in BUNDLED_FONTS {
        root.register_fonts(Blob::new(Arc::new(bytes.to_vec())));
    }
    root.handle_window_event(WindowEvent::EnableAccessTree);
    let (_, update) = root.redraw();
    runs(&update.expect("an accessible tree"))
}

fn runs(update: &TreeUpdate) -> Vec<String> {
    let runs: Vec<&Node> = update
        .nodes
        .iter()
        .map(|(_, node): &(NodeId, Node)| node)
        .filter(|node| node.role() == Role::TextRun)
        .collect();
    assert!(!runs.is_empty(), "the view sets text");
    runs.iter()
        .map(|node| node.font_family().unwrap_or_default().to_owned())
        .collect()
}

/// Wrapped paragraphs, checkbox labels and field placeholders all set Plex Sans, never the
/// toolkit's default family.
#[test]
fn every_interface_text_path_sets_the_interface_family() {
    let ui = FONT_UI[0];
    for (path, runs) in [
        (
            "prose",
            families(prose::<(), ()>(1., "Enter words to search.")),
        ),
        (
            "checkbox",
            families(
                view::checkbox("Reduced motion", false, |_: &mut (), _| {}).typeset(Type::Body, 1.),
            ),
        ),
        (
            "placeholder",
            families(
                view::text_input(String::new(), |_: &mut (), _| {})
                    .typeset(Type::Body, 1.)
                    .placeholder("Search translations"),
            ),
        ),
    ] {
        assert!(
            runs.iter().all(|family| family.contains(ui)),
            "{path} sets {runs:?}, not {ui}"
        );
    }
}
