use super::*;
use crate::attention::PaneMotion;
use crate::tests::support::NoProxy;
use xilem::masonry::theme;
use xilem::{core::DynMessage, masonry::testing::TestHarness};
#[path = "inline/tests.rs"]
mod inline;

#[test]
fn commands_reject_retired_documents_screens_analysis_and_bindings_before_and_after_rebuild() {
    for change in 0..12 {
        let writer = Writer::from_session(Session::examples().unwrap(), None);
        let mut app = App::with_writer(writer);
        if change == 5 {
            app.open_picker(picker::Search::Commands);
        }
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = host(&app, xilem::view::label("Keyboard host"));
        let (pod, mut state) = view.build(&mut ctx, &mut app);
        let mut harness = TestHarness::create_with_size(
            theme::default_property_set(),
            pod.new_widget,
            (300, 100),
        );
        match change {
            0 => app.show_welcome = true,
            1 => app.settings_open = true,
            2 => app.writer = Some(Writer::from_session(Session::examples().unwrap(), None)),
            3 => app
                .writer
                .as_mut()
                .unwrap()
                .session
                .edit(|model| model.refresh_project(Default::default()))
                .unwrap(),
            4 => app
                .preferences
                .config
                .writer
                .shortcuts
                .rebind(
                    WriterCommand::Save,
                    "Primary+Alt+S".to_owned().try_into().unwrap(),
                )
                .unwrap(),
            5 => app.open_picker(picker::Search::Commands),
            6 => app.writer.as_mut().unwrap().toggle_focus(),
            7 => app.preferences.config.ui.keymap = recite_config::Keymap::Vim,
            8 => app.writer.as_mut().unwrap().focus_pane(PaneMotion::Left),
            9 => app.writer.as_mut().unwrap().phase = Phase::ConfirmClose,
            10 => app.writer.as_mut().unwrap().workspaces.screen = crate::workspaces::Screen::Build,
            _ => app.writer.as_mut().unwrap().document_menu = DocumentMenu::Open,
        }
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            Vec::new(),
            DynMessage::new(Invocation {
                action: Action::Command(WriterCommand::Close),
                generation: CommandGeneration::default(),
            }),
        );
        let result =
            harness.edit_root_widget(|root| view.message(&mut state, &mut message, root, &mut app));
        assert!(
            matches!(result, MessageResult::Stale),
            "changed scope {change}"
        );
        *ctx.environment() = message.finish().0;
        let next = host(&app, xilem::view::label("Keyboard host"));
        harness.edit_root_widget(|root| next.rebuild(&view, &mut state, &mut ctx, root, &mut app));
        assert_ne!(state.generation, CommandGeneration::default());
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            Vec::new(),
            DynMessage::new(Invocation {
                action: Action::Command(WriterCommand::Close),
                generation: CommandGeneration::default(),
            }),
        );
        assert!(matches!(
            harness.edit_root_widget(|root| next.message(&mut state, &mut message, root, &mut app)),
            MessageResult::Stale
        ));
        *ctx.environment() = message.finish().0;
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            Vec::new(),
            DynMessage::new(Invocation {
                action: Action::Command(if change == 5 {
                    WriterCommand::Close
                } else {
                    WriterCommand::Settings
                }),
                generation: state.generation,
            }),
        );
        assert!(matches!(
            harness.edit_root_widget(|root| next.message(&mut state, &mut message, root, &mut app)),
            MessageResult::Action(())
        ));
        if change == 5 {
            assert!(app.picker.is_none());
        } else if change == 9 {
            assert!(!app.settings_open);
            assert!(app.status.contains("Choose how to close"));
        } else {
            assert!(app.settings_open);
        }
    }
}
