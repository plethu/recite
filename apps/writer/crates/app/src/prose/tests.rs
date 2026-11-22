use super::{CheckGeneration, Checker, Request};
use crate::Writer;
use crate::test_support::open;
use recite_writer_session::messages::{MsgId, text};
use std::error::Error;
use std::fs;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

const SOURCE: &str = ":: start default\n> speaker=rhea @intro\n  The lantern flickerd.\n-> END\n";
const PROSE: &str = "[prose]\nenabled = true\ndictionaries = [\"words.txt\"]\n";

fn project(prose: &str) -> Result<tempfile::TempDir, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        format!("format_version = 1\n[project]\ncontent_set = \"test\"\nversion = \"1\"\n{prose}"),
    )?;
    fs::write(directory.path().join("scene.recite"), SOURCE)?;
    Ok(directory)
}

/// A Writer bound to a prose channel, as the frontend's worker binds it.
fn writer(
    prose: &str,
) -> Result<(tempfile::TempDir, Writer, UnboundedReceiver<Request>), Box<dyn Error>> {
    let directory = project(prose)?;
    let mut writer = open(&directory)?;
    let (sender, requests) = unbounded_channel();
    writer.prose.sender = Some(sender);
    Ok((directory, writer, requests))
}

/// Run every queued request on `checker` and deliver its result, as the worker does.
fn finish(writer: &mut Writer, requests: &mut UnboundedReceiver<Request>, checker: &mut Checker) {
    while let Ok(request) = requests.try_recv() {
        writer.accept_prose(request.execute(checker));
    }
}

/// The first prose problem's check generation and index.
fn first_prose(writer: &Writer) -> Option<(CheckGeneration, usize)> {
    writer
        .problems()
        .iter()
        .find_map(|problem| problem.prose.map(|prose| (prose.generation, prose.index)))
}

#[test]
fn enabled_projects_list_prose_findings_as_notes_that_never_mark_beats()
-> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer(PROSE)?;
    let own = writer
        .session
        .model()
        .document()
        .diagnostic_snapshot()
        .len();
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut Checker::default());
    let problems = writer.problems();
    assert_eq!(problems.document_len(), own);
    assert_eq!(problems.len(), own + 1);
    let problem = problems.get(own).ok_or("prose problem")?;
    let prose = problem.prose.ok_or("prose finding")?;
    assert_eq!(prose.finding.text, "flickerd");
    assert!(prose.can_add_word);
    // Notes are listed with the document's problems but never mark a beat.
    assert!(writer.beat_problems().is_empty());
    // An unchanged source is not checked again.
    writer.check_prose_if_needed();
    assert!(requests.try_recv().is_err());
    Ok(())
}

#[test]
fn projects_without_prose_request_nothing() -> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer("")?;
    writer.check_prose_if_needed();
    assert!(requests.try_recv().is_err());
    assert!(
        writer
            .session
            .project()
            .ok_or("project")?
            .files()
            .prose()
            .is_none()
    );
    let own = writer
        .session
        .model()
        .document()
        .diagnostic_snapshot()
        .len();
    assert_eq!(writer.problems().len(), own);
    assert!(
        writer
            .problems()
            .iter()
            .all(|problem| problem.prose.is_none())
    );
    assert!(writer.beat_problems().is_empty());
    Ok(())
}

#[test]
fn replacements_fix_the_text_as_one_undoable_step() -> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer(PROSE)?;
    let mut checker = Checker::default();
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut checker);
    let (generation, index) = first_prose(&writer).ok_or("prose finding")?;
    let (range, replacement) = {
        let problems = writer.problems();
        let problem = problems
            .get(problems.document_len() + index)
            .ok_or("problem")?;
        let fix = &problem.prose.ok_or("prose")?.finding.fixes[0];
        (fix.range.clone(), fix.replacement.clone())
    };
    let mut expected = SOURCE.to_owned();
    expected.replace_range(range, &replacement);
    writer.apply_prose_fix(generation, index, 0);
    assert_eq!(writer.status, text(MsgId::WriterProseApplied));
    assert_eq!(writer.session.model().document().source(), expected);
    // The finding belonged to the replaced source: it is gone and cannot be applied again.
    assert!(first_prose(&writer).is_none());
    writer.apply_prose_fix(generation, index, 0);
    assert_eq!(writer.status, text(MsgId::WriterProseStale));
    assert_eq!(writer.session.model().document().source(), expected);
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut checker);
    assert!(first_prose(&writer).is_none());
    writer.session.edit(|model| model.undo())?;
    assert_eq!(writer.session.model().document().source(), SOURCE);
    // Undo restores the misspelling and, once checked, its finding.
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut checker);
    assert!(first_prose(&writer).is_some());
    Ok(())
}

#[test]
fn added_words_suppress_their_findings() -> Result<(), Box<dyn Error>> {
    let (directory, mut writer, mut requests) = writer(PROSE)?;
    let mut checker = Checker::default();
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut checker);
    let (generation, index) = first_prose(&writer).ok_or("prose finding")?;
    writer.add_prose_word(generation, index);
    assert_eq!(
        fs::read_to_string(directory.path().join("words.txt"))?,
        "flickerd\n"
    );
    finish(&mut writer, &mut requests, &mut checker);
    assert!(first_prose(&writer).is_none());
    assert_eq!(
        writer.problems().len(),
        writer
            .session
            .model()
            .document()
            .diagnostic_snapshot()
            .len()
    );
    Ok(())
}

#[test]
fn results_of_superseded_requests_are_dropped() -> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer(PROSE)?;
    let mut checker = Checker::default();
    writer.check_prose_if_needed();
    let superseded = requests.try_recv()?;
    writer.recheck_prose();
    let newest = requests.try_recv()?;
    writer.accept_prose(superseded.execute(&mut checker));
    assert!(first_prose(&writer).is_none());
    writer.accept_prose(newest.execute(&mut checker));
    assert!(first_prose(&writer).is_some());
    Ok(())
}

#[test]
fn fixes_and_words_are_refused_when_stale_pending_or_unwritable() -> Result<(), Box<dyn Error>> {
    let (directory, mut writer, mut requests) = writer(PROSE)?;
    let mut checker = Checker::default();
    writer.check_prose_if_needed();
    finish(&mut writer, &mut requests, &mut checker);
    assert!(!writer.problems().is_empty());
    let (generation, index) = first_prose(&writer).ok_or("prose finding")?;
    // A finding from another check is never applied.
    writer.add_prose_word(CheckGeneration::default(), index);
    assert_eq!(writer.status, text(MsgId::WriterProseStale));
    // A word the dictionary already lists is reported rather than appended twice.
    fs::write(directory.path().join("words.txt"), "flickerd\n")?;
    writer.add_prose_word(generation, index);
    assert_eq!(
        fs::read_to_string(directory.path().join("words.txt"))?,
        "flickerd\n"
    );
    assert_ne!(writer.status, text(MsgId::WriterProseStale));
    // A dictionary that cannot be written reports why.
    fs::remove_file(directory.path().join("words.txt"))?;
    fs::create_dir(directory.path().join("words.txt"))?;
    let before = writer.status.clone();
    writer.add_prose_word(generation, index);
    assert_ne!(writer.status, before);
    assert!(directory.path().join("words.txt").is_dir());
    // A pending draft must be applied or discarded before a replacement.
    writer.change(|model| {
        model.set_draft("Pending typing.".into());
        Ok(())
    });
    writer.apply_prose_fix(generation, index, 0);
    assert_eq!(writer.status, text(MsgId::WriterProseDraftPending));
    assert_eq!(writer.session.model().document().source(), SOURCE);
    Ok(())
}

#[test]
fn closed_workers_and_unrequested_results_change_nothing() -> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer(PROSE)?;
    writer.check_prose_if_needed();
    let checked = requests.try_recv()?.execute(&mut Checker::default());
    assert!(format!("{checked:?}").contains("ProseChecked"));
    // A result that arrives after its request was withdrawn is dropped.
    writer.prose.requested = None;
    writer.accept_prose(checked);
    assert!(first_prose(&writer).is_none());
    // A stopped worker receives nothing and the request stays unsent.
    drop(requests);
    writer.check_prose_if_needed();
    assert!(writer.prose.requested.is_none());
    Ok(())
}

#[test]
fn unchanged_text_in_a_new_allocation_keeps_its_findings() -> Result<(), Box<dyn Error>> {
    let (_directory, mut writer, mut requests) = writer(PROSE)?;
    writer.check_prose_if_needed();
    let mut checked = requests.try_recv()?.execute(&mut Checker::default());
    // The checked text arrives in a copy of the requested allocation, as after a save.
    checked.source = std::sync::Arc::from(SOURCE);
    writer.accept_prose(checked);
    assert!(first_prose(&writer).is_some());
    // The request's own copy moves too; the same text sends no new request.
    if let Some(stamp) = &mut writer.prose.requested {
        stamp.source = std::sync::Arc::from(SOURCE);
    }
    writer.check_prose_if_needed();
    assert!(requests.try_recv().is_err());
    assert!(first_prose(&writer).is_some());
    Ok(())
}

#[test]
fn a_project_that_stops_checking_prose_drops_its_findings() -> Result<(), Box<dyn Error>> {
    let (_enabled, mut checking, mut requests) = writer(PROSE)?;
    checking.check_prose_if_needed();
    let request = requests.try_recv()?;
    let stamp = super::Stamp {
        configuration: request.configuration.clone(),
        source: request.source.clone(),
    };
    let checked = request.execute(&mut Checker::default());
    // The same Writer state after the project's `[prose]` table was removed.
    let (_disabled, mut writer, _requests) = writer("")?;
    writer.prose.requested = Some(stamp);
    writer.prose.checked = Some(checked);
    writer.check_prose_if_needed();
    assert!(writer.prose.requested.is_none());
    assert!(writer.prose.checked.is_none());
    Ok(())
}
