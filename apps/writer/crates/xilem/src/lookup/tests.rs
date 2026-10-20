use super::*;
use crate::{navigation::worker, tests::support};
use tokio::sync::mpsc::{self, UnboundedReceiver};

const SOURCE: &str = ":: part_a default\n> first@11111111111111111111\n  First.\n-> END\n:: part_b\n> second@22222222222222222222\n  Second.\n-> END\n:: part_c\n> third@33333333333333333333\n  Third.\n-> END\n";
struct Fixture {
    writer: Writer,
    jobs: UnboundedReceiver<worker::Job>,
    _runtime: tokio::runtime::Runtime,
    _directory: tempfile::TempDir,
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = support::project(SOURCE)?;
    let mut writer = support::open(&directory)?;
    let (sender, jobs) = mpsc::unbounded_channel();
    let runtime = tokio::runtime::Runtime::new()?;
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    Ok(Fixture {
        writer,
        jobs,
        _runtime: runtime,
        _directory: directory,
    })
}
fn begin(writer: &mut Writer, beat: &str) {
    writer.lookup(
        "part".into(),
        targets::Target {
            document: "scene.recite".into(),
            beat: Some(beat.into()),
        },
    );
}
fn accepted(writer: &Writer) -> Option<&str> {
    writer
        .lookup
        .as_ref()
        .and_then(|search| search.accepted.beat.as_deref())
}

#[test]
fn repeat_advances_only_after_publication_wraps_and_busy_commands_do_not_skip()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let writer = &mut fixture.writer;
    begin(writer, "part_a");
    assert_eq!(accepted(writer), None);
    support::finish_navigation(writer, &mut fixture.jobs)?;
    for (forward, expected) in [
        (true, "part_b"),
        (false, "part_a"),
        (false, "part_c"),
        (true, "part_a"),
    ] {
        let previous = accepted(writer).map(str::to_owned);
        writer.repeat_lookup(forward);
        writer.repeat_lookup(forward);
        assert_eq!(accepted(writer), previous.as_deref());
        support::finish_navigation(writer, &mut fixture.jobs)?;
        assert!(fixture.jobs.try_recv().is_err());
        assert_eq!(accepted(writer), Some(expected));
        assert_eq!(
            writer.session.model().selected_block()?.as_deref(),
            Some(expected)
        );
    }
    Ok(())
}

#[test]
fn rejected_and_cancelled_search_navigation_preserve_the_last_accepted_search()
-> Result<(), Box<dyn std::error::Error>> {
    for cancel in [false, true] {
        let mut fixture = fixture()?;
        let writer = &mut fixture.writer;
        begin(writer, "part_a");
        support::finish_navigation(writer, &mut fixture.jobs)?;
        writer.select(
            writer.session.model().document().passage_snapshot()?[0]
                .id
                .clone(),
        );
        support::finish_navigation(writer, &mut fixture.jobs)?;
        writer.repeat_lookup(true);
        let completed = fixture.jobs.try_recv()?.execute();
        if cancel {
            writer.cancel_navigation();
        } else {
            writer.change(|model| {
                model.set_draft("Newer typing.".into());
                Ok(())
            });
        }
        if let Some(retired) = writer.accept_navigation(completed) {
            retired.retire()?;
        }
        assert_eq!(accepted(writer), Some("part_a"));
        writer.repeat_lookup(true);
        support::finish_navigation(writer, &mut fixture.jobs)?;
        assert_eq!(accepted(writer), Some("part_b"));
    }
    Ok(())
}

#[test]
fn newer_typing_after_implicit_apply_prevents_cursor_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let writer = &mut fixture.writer;
    begin(writer, "part_a");
    support::finish_navigation(writer, &mut fixture.jobs)?;
    writer.select(
        writer.session.model().document().passage_snapshot()?[0]
            .id
            .clone(),
    );
    support::finish_navigation(writer, &mut fixture.jobs)?;
    writer.change(|model| {
        model.set_draft("Captured text.".into());
        Ok(())
    });
    writer.repeat_lookup(true);
    let completed = fixture.jobs.try_recv()?.execute();
    writer.change(|model| {
        model.set_draft("Newer text.".into());
        Ok(())
    });
    if let Some(retired) = writer.accept_navigation(completed) {
        retired.retire()?;
    }
    assert_eq!(accepted(writer), Some("part_a"));
    assert_eq!(writer.session.model().draft(), "Newer text.");
    assert!(fixture.jobs.try_recv().is_err());
    writer.repeat_lookup(true);
    support::finish_navigation(writer, &mut fixture.jobs)?;
    assert_eq!(accepted(writer), Some("part_b"));
    Ok(())
}

#[test]
fn repeat_resolves_accepted_context_after_a_target_is_removed_and_empty_lookup_preserves_search()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let writer = &mut fixture.writer;
    begin(writer, "part_b");
    support::finish_navigation(writer, &mut fixture.jobs)?;
    writer.change(|model| {
        model.select(recite_writer_model::View::Source)?;
        model.set_draft(SOURCE.replace("part_b", "retired"));
        model.apply()?;
        Ok(())
    });
    writer.repeat_lookup(true);
    support::finish_navigation(writer, &mut fixture.jobs)?;
    assert_eq!(accepted(writer), Some("part_a"));
    writer.lookup(
        String::new(),
        targets::Target {
            document: "scene.recite".into(),
            beat: Some("retired".into()),
        },
    );
    support::finish_navigation(writer, &mut fixture.jobs)?;
    assert_eq!(accepted(writer), Some("part_a"));
    writer.repeat_lookup(false);
    support::finish_navigation(writer, &mut fixture.jobs)?;
    assert_eq!(accepted(writer), Some("part_c"));
    Ok(())
}
