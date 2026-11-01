use super::*;
use crate::navigation::worker::Job;
use crate::workspaces::Screen;
use crate::{tests::support, workspaces::tests::ToolDriver};
use recite_core::po::PoEntryId;
use std::fs;
use tokio::sync::mpsc;

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";
const OTHER: &str = ":: elsewhere\n> @rthh7ab\n  A letter.\n-> END\n";
const PO: &str = "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n#. file: other.recite\n#. block: elsewhere\nmsgctxt \"rthh7ab&formal\"\nmsgid \"A letter.\"\nmsgstr \"Un courrier.\"\n\n#. file: scene.recite\n#. block: start\nmsgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";

struct Fixture {
    directory: tempfile::TempDir,
    writer: Writer,
    jobs: mpsc::UnboundedReceiver<Job>,
    _runtime: tokio::runtime::Runtime,
    _tools: ToolDriver,
}

fn fixture(po: &str) -> Fixture {
    let directory = support::project(SOURCE).unwrap();
    fs::write(directory.path().join("other.recite"), OTHER).unwrap();
    fs::write(directory.path().join("fr.po"), po).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut tools = ToolDriver::new(&mut writer);
    writer.open_localisation();
    writer.localisation.path = directory.path().join("fr.po").display().to_string();
    writer.open_catalogue_path();
    tools.finish(&mut writer);
    let id = writer
        .localisation
        .slot
        .document()
        .unwrap()
        .entries()
        .iter()
        .find(|entry| entry.variant() == Some("formal"))
        .unwrap()
        .id();
    writer.inspect_translation(&EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id,
    });
    tools.finish(&mut writer);
    writer.trial.settings.locale = "de".into();
    writer.trial.settings.variant = "earlier".into();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    Fixture {
        directory,
        writer,
        jobs,
        _runtime: runtime,
        _tools: tools,
    }
}

fn key(writer: &Writer) -> EntryKey {
    writer.localisation.entry.as_ref().unwrap().key.clone()
}

#[test]
fn source_actions_cross_files_preserve_po_drafts_and_stage_preview_only_after_success() {
    for action in [SourceAction::Show, SourceAction::Preview] {
        let mut fixture = fixture(PO);
        let writer = &mut fixture.writer;
        let key = key(writer);
        writer.update_translation(&key, 0, "Unsaved translation.".into());
        writer.change(|model| {
            model.set_draft("Applied source draft.".into());
            Ok(())
        });
        writer.open_translation_source(&key, action);
        assert_eq!(writer.trial.settings.locale, "de");
        support::finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert_eq!(
            writer.session.model().document().key().as_str(),
            "other.recite",
            "{}",
            writer.status
        );
        assert_eq!(
            writer.session.model().view(),
            &recite_writer_model::View::Passage("rthh7ab".into())
        );
        assert!(writer.workspaces.screen == Screen::Write);
        assert_eq!(
            writer.trial.visible,
            matches!(action, SourceAction::Preview)
        );
        assert_eq!(
            writer.trial.settings.locale,
            if matches!(action, SourceAction::Preview) {
                "fr"
            } else {
                "de"
            }
        );
        assert_eq!(
            writer.trial.settings.variant,
            if matches!(action, SourceAction::Preview) {
                "formal"
            } else {
                "earlier"
            }
        );
        assert!(
            writer.session.model().preview_page().is_none(),
            "Opening the pane does not restart runtime"
        );
        assert_eq!(
            writer.localisation.slot.draft(key.id).unwrap().forms[0],
            "Unsaved translation."
        );
        assert_eq!(
            fs::read_to_string(fixture.directory.path().join("fr.po")).unwrap(),
            PO
        );
        writer.navigate(Target::Document(
            fixture.directory.path().join("scene.recite"),
        ));
        support::finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert!(
            writer
                .session
                .model()
                .document()
                .source()
                .contains("Applied source draft.")
        );
        assert_eq!(
            fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
            SOURCE
        );
    }
}

#[test]
fn stale_catalogue_or_entry_scope_and_cancellation_reject_apply_and_destination_receipts() {
    for stage in ["apply", "destination"] {
        for changed in ["typing", "entry", "catalogue", "screen", "cancel"] {
            let mut fixture = fixture(PO);
            let writer = &mut fixture.writer;
            let key = key(writer);
            let original_view = writer.session.model().view().clone();
            writer.change(|model| {
                model.set_draft("Captured source draft.".into());
                Ok(())
            });
            writer.open_translation_source(&key, SourceAction::Preview);
            let mut completed = fixture.jobs.try_recv().unwrap().execute();
            if stage == "destination" {
                writer
                    .accept_navigation(completed)
                    .unwrap()
                    .retire()
                    .unwrap();
                completed = fixture.jobs.try_recv().unwrap().execute();
            }
            match changed {
                "typing" => writer.update_translation(&key, 0, "Newer PO draft.".into()),
                "entry" => {
                    let id = writer
                        .localisation
                        .slot
                        .document()
                        .unwrap()
                        .entries()
                        .iter()
                        .find(|entry| entry.context() == Some("ffee6p8"))
                        .unwrap()
                        .id();
                    writer.localisation.entry.as_mut().unwrap().key.id = id;
                }
                "catalogue" => writer
                    .localisation
                    .slot
                    .owner_mut()
                    .unwrap()
                    .renew_identity(),
                "screen" => writer.return_to_writing(),
                "cancel" => writer.cancel_navigation(),
                _ => unreachable!(),
            }
            writer
                .accept_navigation(completed)
                .unwrap()
                .retire()
                .unwrap();
            assert!(
                !writer.navigation.busy(),
                "{stage}/{changed}: {}",
                writer.status
            );
            assert!(fixture.jobs.try_recv().is_err());
            assert_eq!(
                writer.session.model().document().key().as_str(),
                "scene.recite"
            );
            assert_eq!(writer.session.model().view(), &original_view);
            assert_eq!(writer.trial.settings.locale, "de");
            assert_eq!(writer.trial.settings.variant, "earlier");
            assert_eq!(
                writer
                    .session
                    .model()
                    .document()
                    .source()
                    .contains("Captured source draft."),
                stage == "destination"
            );
            if stage == "apply" {
                assert_eq!(writer.session.model().draft(), "Captured source draft.");
            }
            if changed == "typing" {
                assert_eq!(
                    writer.localisation.slot.draft(key.id).unwrap().forms[0],
                    "Newer PO draft."
                );
            }
        }
    }
}

#[test]
fn failed_destinations_and_invalid_language_leave_staged_settings_and_translation_open() {
    for po in [
        PO.replace("rthh7ab&formal", "unknown_destination&formal"),
        PO.replace("file: other.recite", "file: absent.recite"),
        PO.replace("Language: fr", "Language: not/a/locale"),
    ] {
        let mut fixture = fixture(&po);
        let writer = &mut fixture.writer;
        let key = key(writer);
        let original_view = writer.session.model().view().clone();
        writer.open_translation_source(&key, SourceAction::Preview);
        support::finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert_eq!(
            writer.session.model().view(),
            &original_view,
            "{po}: {}",
            writer.status
        );
        assert_eq!(
            writer.session.model().document().key().as_str(),
            "scene.recite"
        );
        assert_eq!(writer.trial.settings.locale, "de");
        assert_eq!(writer.trial.settings.variant, "earlier");
        assert!(writer.workspaces.screen == Screen::Localisation);
        assert!(!writer.trial.visible);
    }
}

#[test]
fn stale_field_callbacks_and_recovered_source_cannot_bypass_navigation_guards() {
    let mut fixture = fixture(PO);
    let writer = &mut fixture.writer;
    let mut stale = key(writer);
    stale.id = PoEntryId::new(999);
    writer.open_translation_source(&stale, SourceAction::Show);
    assert!(!writer.navigation.busy());
    assert!(fixture.jobs.try_recv().is_err());
    assert!(writer.workspaces.screen == Screen::Localisation);
    let key = key(writer);
    writer.change(|model| {
        model.select(recite_writer_model::View::Source)?;
        model.stage_repaired_source("Unfinished recovered source.".to_owned().into())
    });
    writer.open_translation_source(&key, SourceAction::Preview);
    assert!(!writer.navigation.busy());
    assert!(fixture.jobs.try_recv().is_err());
    assert!(writer.status.contains("Repair and import"));
    assert_eq!(
        writer.session.model().draft(),
        "Unfinished recovered source."
    );
}
