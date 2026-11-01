use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn page(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    keyed((), specimen(app, 600.))
}

fn font_sizes(mounted: &Mounted<App, impl WidgetView<App, Widget = Passthrough>>) -> Vec<f32> {
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut sizes = Vec::new();
    while let Some(node) = pending.pop() {
        if node
            .value()
            .is_some_and(|value| value.contains("The last tram waits."))
            && let Some(size) = node.data().font_size()
        {
            sizes.push(size);
        }
        pending.extend(node.children());
    }
    sizes
}

#[test]
fn specimen_matches_the_independent_manuscript_size_at_both_ui_scales() {
    let mut mounted = Mounted::new(App::empty(), page);
    assert_eq!(font_sizes(&mounted), [20.]);
    let presentation = mounted.state.preferences.config.writer.presentation;
    mounted.state.preferences.config.writer.presentation = presentation
        .with_value(WriterPresentationField::UiScale, 200)
        .unwrap();
    mounted.settle();
    assert_eq!(font_sizes(&mounted), [20.]);
    mounted.state.preferences.config.writer.presentation = presentation
        .with_value(WriterPresentationField::UiScale, 200)
        .unwrap()
        .with_value(WriterPresentationField::ReadingSize, 25)
        .unwrap();
    mounted.settle();
    assert_eq!(font_sizes(&mounted), [25.]);
}
