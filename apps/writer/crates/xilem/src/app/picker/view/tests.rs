use super::*;
use crate::app::view as app_view;
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use xilem::masonry::{accesskit::Role, widgets::Passthrough};

fn surface(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    keyed((), app_view::app(app))
}

#[test]
fn search_keeps_the_document_visible_and_disables_background_actions() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let app = App::with_writer(support::open(&directory).unwrap());
    let mut mounted = Mounted::new(app, surface);
    mounted.replace_text("Manuscript", "A writer keeps their place.");
    let original = mounted.find("Manuscript");
    mounted.state.open_picker(super::super::Search::GoTo);
    mounted.settle();
    let field = mounted.harness.get_widget_with_id(original);
    assert!(
        !field.ctx().is_stashed(),
        "the source remains visible behind search"
    );
    assert!(field.ctx().is_disabled());
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut dialog = false;
    while let Some(node) = pending.pop() {
        dialog |= node.role() == Role::Dialog && node.data().is_modal();
        pending.extend(node.children());
    }
    assert!(dialog, "native accessibility identifies the active dialog");
    mounted.activate("Save");
    assert!(mounted.state.picker.is_some());
    assert_eq!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .draft(),
        "A writer keeps their place."
    );
    mounted.state.dismiss_picker();
    mounted.settle();
    assert_eq!(mounted.find("Manuscript"), original);
    assert!(
        !mounted
            .harness
            .get_widget_with_id(original)
            .ctx()
            .is_disabled()
    );
}

#[test]
fn changing_search_scope_keeps_query_and_native_field_but_retires_result_authority() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.open_picker(super::super::Search::GoTo);
    let mut mounted = Mounted::new(app, surface);
    mounted.replace_text("Search query", "tell");
    let field = mounted.find("Search query");
    let instance = mounted.state.picker.as_ref().unwrap().instance.clone();
    mounted.activate("Saved project");
    let picker = mounted.state.picker.as_ref().unwrap();
    assert_eq!(picker.query, "tell");
    assert!(
        picker.instance != instance,
        "queued actions from the previous scope cannot select new results"
    );
    assert_eq!(mounted.find("Search query"), field);
    assert_eq!(picker.mode(), Mode::ProjectText);
    assert!(!picker.matches.is_empty());
    mounted.activate("This beat");
    assert_eq!(
        mounted.state.picker.as_ref().unwrap().mode(),
        Mode::Passages
    );
    assert_eq!(mounted.state.picker.as_ref().unwrap().query, "tell");
    assert_eq!(mounted.find("Search query"), field);
}

#[test]
fn current_beat_search_includes_visible_text_without_publishing_it() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.writer.as_mut().unwrap().change(|model| {
        model.set_draft("Copper coils are humming.".into());
        Ok(())
    });
    app.open_picker(super::super::Search::Passages);
    app.filter_picker("copper".into());
    assert_eq!(app.picker.as_ref().unwrap().matches.len(), 1);
    app.switch_search(super::super::Search::ProjectText);
    assert!(app.picker.as_ref().unwrap().matches.is_empty());
    assert_eq!(app.picker.as_ref().unwrap().query, "copper");
    app.dismiss_picker();
    assert_eq!(
        app.writer.as_ref().unwrap().session.model().draft(),
        "Copper coils are humming."
    );
    assert!(
        !std::fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Copper")
    );
}
