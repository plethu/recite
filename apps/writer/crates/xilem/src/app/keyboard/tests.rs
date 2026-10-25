use super::*;
use crate::attention::PaneMotion;
use crate::tests::support;
use crate::tests::support::NoProxy;
use tokio::sync::mpsc;
use xilem::masonry::theme;
use xilem::{core::DynMessage, masonry::testing::TestHarness};
const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Original.\n> next@22222222222222222222\n  Afterwards.\n-> END\n";

#[path = "inline/tests.rs"]
mod inline;
#[path = "opening/tests.rs"]
mod opening;

#[test]
fn commands_reject_retired_documents_screens_analysis_and_bindings_before_and_after_rebuild() {
    for change in 0..14 {
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
            12 => app
                .writer
                .as_mut()
                .unwrap()
                .session
                .edit(|model| model.select(recite_writer_model::View::Source))
                .unwrap(),
            13 => {
                let writer = app.writer.as_mut().unwrap();
                let target = writer
                    .session
                    .model()
                    .document()
                    .passage_snapshot()
                    .unwrap()[1]
                    .id
                    .clone();
                writer
                    .session
                    .edit(|model| model.select(recite_writer_model::View::Passage(target)))
                    .unwrap();
            }
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

#[test]
fn queued_save_survives_focus_acknowledgement_and_context_refresh_without_losing_disk_work() {
    for blur in [false, true] {
        for rebuild in [false, true] {
            let directory = support::project(SOURCE).unwrap();
            let mut writer = support::open(&directory).unwrap();
            writer.change(|model| {
                model.set_draft("Accepted unsaved prose.".into());
                model.apply()
            });
            let target = EditingTarget::capture(&writer).unwrap();
            if blur {
                writer.attention.set_field(Some(target.clone()));
            } else {
                writer.phase = Phase::ConfirmClose;
                writer.native_field_focus(Some(EditingTarget::passage(
                    &writer,
                    "22222222222222222222".into(),
                )));
                writer.phase = Phase::Writing;
                assert_eq!(
                    writer.field_restoration(),
                    Some(FieldRestoration::Passage(target.clone()))
                );
            }
            let mut app = App::with_writer(writer);
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
            let generation = state.generation;
            let (sender, mut work) = mpsc::unbounded_channel();
            app.writer.as_mut().unwrap().work.sender = Some(sender);
            let mut message = MessageCtx::new(
                std::mem::take(ctx.environment()),
                Vec::new(),
                DynMessage::new(Invocation {
                    action: Action::FieldFocus((!blur).then_some(target)),
                    generation,
                }),
            );
            assert!(matches!(
                harness.edit_root_widget(|root| view.message(
                    &mut state,
                    &mut message,
                    root,
                    &mut app
                )),
                MessageResult::Action(())
            ));
            *ctx.environment() = message.finish().0;
            assert!(app.writer.as_ref().unwrap().field_restoration().is_none());
            app.writer.as_mut().unwrap().request_editing();
            let next = host(&app, xilem::view::label("Keyboard host"));
            if rebuild {
                harness.edit_root_widget(|root| {
                    next.rebuild(&view, &mut state, &mut ctx, root, &mut app)
                });
                let context = harness
                    .root_widget()
                    .get_prop::<KeyboardContext>()
                    .0
                    .clone()
                    .unwrap();
                assert_eq!(context.generation, generation);
                assert_eq!(context.attention.field().is_none(), blur);
                assert_eq!(
                    context.attention.edit_request(),
                    app.writer.as_ref().unwrap().attention.edit_request()
                );
            }
            let mut message = MessageCtx::new(
                std::mem::take(ctx.environment()),
                Vec::new(),
                DynMessage::new(Invocation {
                    action: Action::Command(WriterCommand::Save),
                    generation,
                }),
            );
            let current = if rebuild { &next } else { &view };
            assert!(matches!(
                harness.edit_root_widget(|root| current.message(
                    &mut state,
                    &mut message,
                    root,
                    &mut app
                )),
                MessageResult::Action(())
            ));
            let writer = app.writer.as_mut().unwrap();
            support::finish_work(writer, &mut work).unwrap();
            assert_eq!(
                std::fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
                SOURCE.replace("Original.", "Accepted unsaved prose.")
            );
            assert!(!writer.session.dirty());
        }
    }
}

#[test]
fn clearing_refused_focus_retires_its_request_without_retargeting_a_queued_save() {
    for rebuild in [false, true] {
        let directory = support::project(SOURCE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.change(|model| {
            model.set_draft("Accepted unsaved prose.".into());
            model.apply()?;
            model.select(recite_writer_model::View::Block("start".into()))
        });
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        writer.show_workspace(crate::map::Workspace::Map);
        assert!(writer.navigation.busy());
        writer.native_field_focus(Some(EditingTarget::passage(
            &writer,
            "11111111111111111111".into(),
        )));
        assert_eq!(writer.field_restoration(), Some(FieldRestoration::Clear));
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
        let mut app = App::with_writer(writer);
        let mut ctx = ViewCtx::new(Arc::new(NoProxy), runtime);
        let view = host(&app, xilem::view::label("Keyboard host"));
        let (pod, mut state) = view.build(&mut ctx, &mut app);
        let mut harness = TestHarness::create_with_size(
            theme::default_property_set(),
            pod.new_widget,
            (300, 100),
        );
        let generation = state.generation;
        let (sender, mut work) = mpsc::unbounded_channel();
        app.writer.as_mut().unwrap().work.sender = Some(sender);
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            Vec::new(),
            DynMessage::new(Invocation {
                action: Action::FieldFocus(None),
                generation,
            }),
        );
        assert!(matches!(
            harness.edit_root_widget(|root| view.message(&mut state, &mut message, root, &mut app)),
            MessageResult::Action(())
        ));
        *ctx.environment() = message.finish().0;
        assert!(app.writer.as_ref().unwrap().field_restoration().is_none());
        let next = host(&app, xilem::view::label("Keyboard host"));
        if rebuild {
            harness
                .edit_root_widget(|root| next.rebuild(&view, &mut state, &mut ctx, root, &mut app));
            assert_eq!(state.generation, generation);
            assert!(
                harness
                    .root_widget()
                    .get_prop::<KeyboardContext>()
                    .0
                    .as_ref()
                    .unwrap()
                    .field_restoration
                    .is_none()
            );
        }
        let mut message = MessageCtx::new(
            std::mem::take(ctx.environment()),
            Vec::new(),
            DynMessage::new(Invocation {
                action: Action::Command(WriterCommand::Save),
                generation,
            }),
        );
        let current = if rebuild { &next } else { &view };
        assert!(matches!(
            harness.edit_root_widget(|root| current.message(
                &mut state,
                &mut message,
                root,
                &mut app
            )),
            MessageResult::Action(())
        ));
        let writer = app.writer.as_mut().unwrap();
        support::finish_work(writer, &mut work).unwrap();
        assert_eq!(
            writer.session.model().view(),
            &recite_writer_model::View::Block("start".into())
        );
        assert!(!writer.session.model().has_draft());
        assert!(!writer.session.dirty());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
            SOURCE.replace("Original.", "Accepted unsaved prose.")
        );
    }
}
