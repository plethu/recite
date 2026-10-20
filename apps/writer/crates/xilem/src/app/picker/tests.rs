use super::*;
use crate::tests::support::{self, NoProxy};
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
};

#[test]
fn go_to_applies_draft_uses_the_worker_and_preserves_cross_document_selection() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    std::fs::write(
        directory.path().join("other.recite"),
        ":: other default\n-> END\n",
    )
    .unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = tokio::sync::mpsc::unbounded_channel();
    let writer = app.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.change(|model| {
        model.set_draft("Text before lookup.".into());
        Ok(())
    });
    app.open_picker(Mode::GoTo);
    app.filter_picker("other".into());
    assert_eq!(app.picker.as_ref().unwrap().matches.len(), 2);
    app.step_picker(true);
    let index = app.picker.as_ref().unwrap().active;
    app.activate_picker(index);
    assert!(app.picker.is_none());
    let writer = app.writer.as_mut().unwrap();
    assert!(writer.navigation.busy());
    support::finish_navigation(writer, &mut jobs).unwrap();
    assert_eq!(
        writer.session.model().document().key().as_str(),
        "other.recite"
    );
    assert_eq!(
        writer.session.model().selected_block().unwrap().as_deref(),
        Some("other")
    );
    writer.switch(&directory.path().join("scene.recite"));
    support::finish_navigation(writer, &mut jobs).unwrap();
    assert!(!writer.session.model().has_draft());
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Text before lookup.")
    );
}

#[test]
fn vim_alias_runs_the_shared_save_path_and_unavailable_commands_leave_search_open() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.preferences.config.ui.keymap = recite_config::Keymap::Vim;
    let (sender, mut saves) = tokio::sync::mpsc::unbounded_channel();
    let writer = app.writer.as_mut().unwrap();
    writer.work.sender = Some(sender);
    writer.change(|model| {
        model.set_draft("Save from command search.".into());
        Ok(())
    });
    app.open_picker(Mode::Commands);
    app.filter_picker("Map".into());
    app.activate_picker(0);
    assert!(app.picker.is_some());
    app.filter_picker(":w".into());
    assert_eq!(app.picker.as_ref().unwrap().matches.len(), 1);
    app.activate_picker(0);
    assert!(app.picker.is_none());
    let writer = app.writer.as_mut().unwrap();
    assert!(writer.work.busy());
    support::finish_work(writer, &mut saves).unwrap();
    assert!(
        std::fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Save from command search.")
    );
}

#[test]
fn saved_text_search_excludes_field_drafts_and_empty_search_has_no_results() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    let original = app
        .writer
        .as_ref()
        .unwrap()
        .session
        .model()
        .draft()
        .to_owned();
    app.writer.as_mut().unwrap().change(|model| {
        model.set_draft("Unsearchable field draft.".into());
        Ok(())
    });
    app.open_picker(Mode::ProjectText);
    assert_eq!(app.picker.as_ref().unwrap().total, 0);
    app.filter_picker("Unsearchable".into());
    assert_eq!(app.picker.as_ref().unwrap().total, 0);
    app.filter_picker(original.split_whitespace().next().unwrap().into());
    assert!(app.picker.as_ref().unwrap().total > 0);
    app.dismiss_picker();
    assert_eq!(
        app.writer.as_ref().unwrap().session.model().draft(),
        "Unsearchable field draft."
    );
}

#[test]
fn native_picker_renders_bounded_results_at_personal_scales() {
    for scale in [100, 200] {
        let writer = Writer::from_session(Session::examples().unwrap(), None);
        let mut app = App::with_writer(writer);
        app.preferences.config.writer.presentation = app
            .preferences
            .config
            .writer
            .presentation
            .with_value(recite_config::WriterPresentationField::UiScale, scale)
            .unwrap();
        app.theme.refresh(&app.preferences.config.writer);
        for mode in [Mode::Commands, Mode::GoTo] {
            app.open_picker(mode);
            let mut ctx = ViewCtx::new(
                Arc::new(NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let view = super::super::view::app(&mut app);
            let (pod, _) = view.build(&mut ctx, &mut app);
            let mut harness = TestHarness::create_with_size(
                app.theme.initial_properties(),
                NewWidget::new(SizedBox::new(pod.new_widget)),
                (1280, 800),
            );
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let mut buttons = 0;
            let mut query = false;
            while let Some(node) = pending.pop() {
                buttons += usize::from(
                    node.role() == xilem::masonry::accesskit::Role::Button && !node.is_disabled(),
                );
                query |= node.label().as_deref() == Some("Search query");
                pending.extend(node.children());
            }
            assert!(query);
            assert!(buttons <= 9, "Picker must mount only one page of results");
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_PICKER") {
                harness
                    .render()
                    .save(format!("{prefix}-{mode:?}-{scale}.png"))
                    .unwrap();
            }
        }
    }
}
