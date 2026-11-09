#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::picker;
use crate::session::Session;
use recite_config::WriterCommand;
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn rename_beat_choices_keep_native_result_cost_bounded_as_the_scene_grows() {
    let mut baseline = None;
    let mut whole_app_baseline = None;
    for count in [8, 40, 1024, 1040] {
        let mut source = ":: start default\n-> END\n".to_owned();
        for index in 1..count {
            source.push_str(&format!(":: beat_{index:04}\n-> END\n"));
        }
        let directory = support::project(&source).unwrap();
        let mut app = App::with_writer(support::open(&directory).unwrap());
        let writer = app.writer.as_mut().unwrap();
        writer.workspaces.screen = crate::workspaces::Screen::Rename;
        writer.choose_rename_beat();
        app.open_editor_choices();
        assert_eq!(app.picker.as_ref().unwrap().total, count);
        assert_eq!(app.picker.as_ref().unwrap().mode(), Mode::RenameBeat);
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = super::super::super::view::app(&mut app);
        let (pod, _) = view.build(&mut ctx, &mut app);
        let harness = TestHarness::create_with_size(
            app.theme.initial_properties(),
            NewWidget::new(SizedBox::new(pod.new_widget)),
            (1280, 800),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut nodes = 0;
        let mut buttons = 0;
        while let Some(node) = pending.pop() {
            nodes += 1;
            buttons += usize::from(node.role() == accesskit::Role::Button && !node.is_disabled());
            pending.extend(node.children());
        }
        assert!(buttons <= 9, "Rename choices must mount only one page");
        if matches!(count, 40 | 1040) {
            assert_eq!(
                *whole_app_baseline.get_or_insert(nodes),
                nodes,
                "retained workspace and dialog stay bounded once sidebar pagination is present"
            );
        }
        let mut pending = vec![tree.state().root()];
        let dialog = loop {
            let node = pending.pop().expect("native chooser dialog");
            if node.role() == accesskit::Role::Dialog {
                break node;
            }
            pending.extend(node.children());
        };
        let mut pending = vec![dialog];
        let mut result_nodes = 0;
        while let Some(node) = pending.pop() {
            result_nodes += 1;
            pending.extend(node.children());
        }
        assert_eq!(
            *baseline.get_or_insert(result_nodes),
            result_nodes,
            "result cost remains constant across the sidebar pagination boundary"
        );
        app.filter_picker("beat_0007".into());
        assert_eq!(app.picker.as_ref().unwrap().total, 1);
        app.activate_picker(0);
        assert!(app.picker.is_none());
        let writer = app.writer.as_ref().unwrap();
        assert_eq!(
            writer.session.model().selected_block().unwrap().as_deref(),
            Some("start")
        );
        assert_eq!(writer.session.model().document().source(), source);
    }
}
