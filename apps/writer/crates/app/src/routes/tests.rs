use super::*;
pub use crate::navigation::{self, Target};
use crate::test_support as support;
use recite_writer_model::{EditOperation, View};
use tokio::sync::mpsc as async_mpsc;
const SOURCE: &str = "# Keep comments.\n:: start default\n? @rthh7ab\n  Let me through.\n  -> accepted\n:: accepted\n-> END\n:: alternative\n-> END\n";
struct Fixture {
    directory: tempfile::TempDir,
    writer: Writer,
    jobs: async_mpsc::UnboundedReceiver<navigation::worker::Job>,
    _runtime: tokio::runtime::Runtime,
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = support::project(SOURCE)?;
    let mut writer = support::open(&directory)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (sender, jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    Ok(Fixture {
        directory,
        writer,
        jobs,
        _runtime: runtime,
    })
}
#[test]
fn destination_choice_applies_prose_first_and_uses_one_separate_structural_undo_step() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.change(|model| {
        model.set_draft("New reply prose.".into());
        Ok(())
    });
    writer.choose_destination(RouteOwner::Reply("rthh7ab".into()));
    let crate::choices::Request::Destination(request) = writer.pending_choice.take().unwrap()
    else {
        panic!("destination request")
    };
    request.apply(writer, "alternative".into());
    assert_eq!(writer.session.model().document().source(), SOURCE);
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("-> alternative")
    );
    assert_eq!(writer.session.model().draft(), "New reply prose.");
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    writer.navigate(Target::Edit(EditOperation::Undo));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("-> accepted")
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("New reply prose.")
    );
    writer.navigate(Target::Edit(EditOperation::Undo));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().document().source(), SOURCE);
}
#[test]
fn stale_or_undeclared_destination_receipts_do_not_apply_a_draft() {
    for invalid in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.choose_destination(RouteOwner::Reply("rthh7ab".into()));
        let crate::choices::Request::Destination(request) = writer.pending_choice.take().unwrap()
        else {
            panic!("destination request")
        };
        if !invalid {
            writer.change(|model| {
                model.set_draft("Newer draft.".into());
                Ok(())
            });
        }
        let before = writer.session.model().recovery();
        request.apply(writer, if invalid { "not_a_beat" } else { "END" }.into());
        assert_eq!(writer.session.model().recovery(), before);
        assert_eq!(writer.session.model().document().source(), SOURCE);
        assert!(!writer.navigation.busy());
    }
}
#[test]
fn cancelled_or_newer_typing_structural_jobs_preserve_source_selection_and_history() {
    for cancel in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.navigate(Target::Structure(StructuralCommand::AddBeat));
        let completed = fixture.jobs.try_recv().unwrap().execute();
        if cancel {
            writer.cancel_navigation();
        } else {
            writer.change(|model| {
                model.set_draft("Newer draft.".into());
                Ok(())
            });
        }
        let recovery = writer.session.model().recovery();
        drop(writer.accept_navigation(completed));
        assert_eq!(writer.session.model().recovery(), recovery);
        assert_eq!(writer.session.model().document().source(), SOURCE);
        assert!(!writer.session.model().document().can_undo());
        assert_eq!(
            writer.session.model().view(),
            &View::Passage("rthh7ab".into())
        );
    }
}

#[test]
fn speaker_choice_offers_the_scene_speakers_and_changes_one_line() {
    let directory = support::project(
        ":: start default\n> speaker=mara @a1\n  First.\n\n> speaker=ivo @a2\n  Second.\n-> END\n",
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.choose_speaker("a1".into());
    let crate::choices::Request::Speaker(request) = writer.pending_choice.take().unwrap() else {
        panic!("speaker request")
    };
    let offered = request
        .choices
        .iter()
        .map(|choice| choice.value.as_str())
        .collect::<Vec<_>>();
    assert!(
        offered.contains(&"mara") && offered.contains(&"ivo"),
        "{offered:?}"
    );
    request.apply(&mut writer, "ivo".into());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    let source = writer.session.model().document().source().to_owned();
    assert!(source.contains("> speaker=ivo @a1"), "{source}");
    assert!(source.contains("> speaker=ivo @a2"), "{source}");
    // A speaker the choice did not offer changes nothing.
    writer.choose_speaker("a2".into());
    let crate::choices::Request::Speaker(request) = writer.pending_choice.take().unwrap() else {
        panic!("speaker request")
    };
    request.apply(&mut writer, "nobody".into());
    assert!(!writer.navigation.busy());
    assert_eq!(writer.session.model().document().source(), source);
}
