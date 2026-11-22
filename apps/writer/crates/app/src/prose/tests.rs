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
fn enabled_projects_list_count_and_place_prose_findings() -> Result<(), Box<dyn Error>> {
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
    assert_eq!(
        writer.beat_problems(),
        [("start".to_owned(), problem.diagnostic.message.clone())]
    );
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
