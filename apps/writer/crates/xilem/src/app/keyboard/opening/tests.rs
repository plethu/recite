#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::picker;
use crate::attention::DocumentMenu;
use crate::session::Session;
use crate::{map::Workspace, workspaces::Screen};
use recite_config::WriterCommand;
use recite_writer_model::{View as DocumentView, Workbench};
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;
use xilem::masonry::widgets::Passthrough;

fn deliver<V: WidgetView<App, Widget = Passthrough>>(
    app: &mut App,
    view: &V,
    local: &mut V::ViewState,
    ctx: &mut ViewCtx,
    harness: &mut TestHarness<Passthrough>,
    invocation: Invocation,
) -> MessageResult<()> {
    let mut message = MessageCtx::new(
        std::mem::take(ctx.environment()),
        Vec::new(),
        DynMessage::new(invocation),
    );
    let result = harness.edit_root_widget(|root| view.message(local, &mut message, root, app));
    *ctx.environment() = message.finish().0;
    result
}

#[test]
fn queued_picker_opening_survives_real_save_reconciliation_before_and_after_rebuild() {
    for rebuild in [false, true] {
        let directory = support::project(SOURCE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.change(|model| {
            model.set_draft("Current text after Save.".into());
            Ok(())
        });
        let (sender, mut work) = mpsc::unbounded_channel();
        writer.work.sender = Some(sender);
        writer.save();
        let mut app = App::with_writer(writer);
        let captured = Scope::capture(&app);
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = host(&app, xilem::view::label("Keyboard host"));
        let (pod, mut local) = view.build(&mut ctx, &mut app);
        let invocation = Invocation {
            action: Action::OpenPicker {
                search: picker::Search::GoTo,
                generation: local.generations.picker_opening,
            },
            generation: local.generations.command,
        };
        let mut harness = TestHarness::create_with_size(
            theme::default_property_set(),
            pod.new_widget,
            (300, 100),
        );
        support::finish_work(app.writer.as_mut().unwrap(), &mut work).unwrap();
        assert!(
            std::fs::read_to_string(directory.path().join("scene.recite"))
                .unwrap()
                .contains("Current text after Save.")
        );
        let current = Scope::capture(&app);
        assert!(!captured.same_command_scope(&current));
        assert!(captured.same_picker_opening_scope(&current));
        let next = host(&app, xilem::view::label("Keyboard host"));
        if rebuild {
            harness
                .edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut app));
            assert_ne!(local.generations.command, invocation.generation);
        }
        let result = if rebuild {
            deliver(
                &mut app,
                &next,
                &mut local,
                &mut ctx,
                &mut harness,
                invocation,
            )
        } else {
            deliver(
                &mut app,
                &view,
                &mut local,
                &mut ctx,
                &mut harness,
                invocation,
            )
        };
        assert!(matches!(result, MessageResult::Action(())));
        assert_eq!(app.picker.as_ref().unwrap().mode(), picker::Mode::GoTo);
    }
}

#[test]
fn picker_opening_still_rejects_changed_lifetime_document_selection_screen_picker_and_bindings() {
    for change in 0..13 {
        let directory = support::project(SOURCE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        if change == 1 {
            writer.change(|model| model.select(DocumentView::Source));
        }
        let mut app = App::with_writer(writer);
        if matches!(change, 10 | 11) {
            app.open_picker(picker::Search::Commands);
        }
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = host(&app, xilem::view::label("Keyboard host"));
        let (pod, mut local) = view.build(&mut ctx, &mut app);
        let generation = local.generations.picker_opening;
        let strict = local.generations.command;
        let mut harness = TestHarness::create_with_size(
            theme::default_property_set(),
            pod.new_widget,
            (300, 100),
        );
        match change {
            0 => app.writer.as_mut().unwrap().window = recite_writer_app::WindowKey::next(),
            1 => app.writer.as_mut().unwrap().change(|model| {
                *model = Workbench::open("other.recite", SOURCE)?;
                model.select(DocumentView::Source)
            }),
            2 => app
                .writer
                .as_mut()
                .unwrap()
                .change(|model| model.select(DocumentView::Passage("rthh7ab".into()))),
            3 => app.writer.as_mut().unwrap().workspaces.screen = Screen::Build,
            4 => app.open_picker(picker::Search::Commands),
            5 => {
                app.preferences
                    .config
                    .writer
                    .shortcuts
                    .rebind(
                        WriterCommand::GoTo,
                        "Primary+Alt+P".to_owned().try_into().unwrap(),
                    )
                    .unwrap();
            }
            6 => app.writer.as_mut().unwrap().map.workspace = Workspace::Map,
            7 => recite_writer_app::test_support::set_settings(&mut app.shell, true),
            8 => recite_writer_app::test_support::set_welcome(&mut app.shell, true),
            9 => recite_writer_app::test_support::set_phase(
                app.writer.as_mut().unwrap(),
                Phase::ConfirmClose,
            ),
            10 => app.open_picker(picker::Search::Commands),
            11 => app.step_picker(true),
            _ => app.preferences.config.ui.keymap = recite_config::Keymap::Vim,
        }
        let invocation = || Invocation {
            action: Action::OpenPicker {
                search: picker::Search::GoTo,
                generation,
            },
            generation: strict,
        };
        assert!(
            matches!(
                deliver(
                    &mut app,
                    &view,
                    &mut local,
                    &mut ctx,
                    &mut harness,
                    invocation()
                ),
                MessageResult::Stale
            ),
            "live changed scope {change}"
        );
        let next = host(&app, xilem::view::label("Keyboard host"));
        harness.edit_root_widget(|root| next.rebuild(&view, &mut local, &mut ctx, root, &mut app));
        assert!(
            matches!(
                deliver(
                    &mut app,
                    &next,
                    &mut local,
                    &mut ctx,
                    &mut harness,
                    invocation()
                ),
                MessageResult::Stale
            ),
            "rebuilt changed scope {change}"
        );
    }
}

#[test]
fn accepted_opening_closes_the_document_menu_before_presenting_search() {
    let mut writer = Writer::from_session(Session::examples().unwrap(), None);
    recite_writer_app::test_support::set_document_menu(&mut writer, DocumentMenu::Open);
    let mut app = App::with_writer(writer);
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = host(&app, xilem::view::label("Keyboard host"));
    let (pod, mut local) = view.build(&mut ctx, &mut app);
    let invocation = Invocation {
        action: Action::OpenPicker {
            search: picker::Search::GoTo,
            generation: local.generations.picker_opening,
        },
        generation: local.generations.command,
    };
    let mut harness =
        TestHarness::create_with_size(theme::default_property_set(), pod.new_widget, (300, 100));
    assert!(matches!(
        deliver(
            &mut app,
            &view,
            &mut local,
            &mut ctx,
            &mut harness,
            invocation
        ),
        MessageResult::Action(())
    ));
    assert!(app.picker.is_some());
    app.dismiss_picker();
    assert_eq!(
        app.writer.as_ref().unwrap().document_menu(),
        DocumentMenu::Closed
    );
}
