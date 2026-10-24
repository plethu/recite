use super::*;
use crate::tests::support::{click, labels, open, project};
use std::fmt::Write;

#[test]
fn default_beat_metadata_follows_the_selected_authored_beat() {
    let directory = project(":: start default\n-> later\n:: later\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    assert!(writer.reading.is_default);
    writer.change(|model| model.select(View::Source));
    click(&mut writer, &["Start · Start"]).unwrap();
    assert!(
        writer.reading.is_default,
        "retained block metadata survives Source"
    );
    click(&mut writer, &["Later · end"]).unwrap();
    assert!(!writer.reading.is_default);
    click(&mut writer, &["Start · Start"]).unwrap();
    assert!(writer.reading.is_default);
}

#[test]
fn blank_document_creates_a_beat_and_edits_its_frozen_first_line() {
    let directory = project("# Keep this comment.\n").unwrap();
    let mut writer = open(&directory).unwrap();
    assert_eq!(writer.status, "Ready");
    assert!(
        !labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Repair source draft")
    );
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "No beats yet")
    );
    click(&mut writer, &["Add beat", "Edit passage 1"]).unwrap();
    let first = writer.session.model().view().clone();
    assert!(matches!(first, View::Passage(_)));
    writer.change(|model| {
        model.set_draft("An opening line.".into());
        Ok(())
    });
    click(&mut writer, &["Apply draft", "Add reply"]).unwrap();
    let reply = writer.session.model().view().clone();
    assert_ne!(first, reply);
    let source = writer.session.model().document().source();
    assert!(source.starts_with("# Keep this comment."));
    assert!(source.contains("An opening line."));
    click(&mut writer, &["Undo applied"]).unwrap();
    assert_eq!(
        writer
            .session
            .model()
            .document()
            .passage_snapshot()
            .unwrap()
            .len(),
        1
    );
    click(&mut writer, &["Edit passage 1"]).unwrap();
    assert_eq!(writer.session.model().view(), &first);
    assert_eq!(writer.session.model().draft(), "An opening line.");
}

#[test]
fn empty_and_effect_only_beats_have_distinct_reading_states() {
    let directory = project(":: start default\n").unwrap();
    let mut writer = open(&directory).unwrap();
    click(&mut writer, &["Start · Start"]).unwrap();
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label.starts_with("No dialogue yet."))
    );
    click(&mut writer, &["Add line"]).unwrap();
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Manuscript")
    );
    drop(writer);
    let directory = project(":: start default\n!blocking overlay(work)\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    click(&mut writer, &["Start · Start · end"]).unwrap();
    let labels = labels(&mut writer).unwrap();
    assert!(labels.iter().any(|label| label.starts_with("Effect ·")));
    assert!(labels.iter().any(|label| label == "◆ END ◆"));
    assert!(
        !labels
            .iter()
            .any(|label| label.starts_with("No dialogue yet."))
    );
}

#[test]
fn structural_actions_preserve_rejected_prose_and_offer_external_repair_for_syntax() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let source = writer.session.model().document().source_snapshot();
    let selection = writer.session.model().view().clone();
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    click(
        &mut writer,
        &["Add line", "Add reply", "Project", "Read beat", "Add beat"],
    )
    .unwrap();
    assert_eq!(writer.session.model().view(), &selection);
    assert_eq!(writer.session.model().draft(), "-> syntax");
    assert!(Arc::ptr_eq(
        &source,
        &writer.session.model().document().source_snapshot()
    ));
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    click(&mut writer, &["Apply draft"]).unwrap();
    assert!(writer.status.contains("your editor"));
    assert_eq!(writer.session.model().draft(), "-> syntax");
}

#[test]
fn incomplete_and_ambiguous_headers_remain_visible_without_authorizing_edits() {
    for source in [
        ":: start default\n->\n",
        ":: start default\n> one@11111111111111111111\n  First.\n-> END\n:: start\n> two@22222222222222222222\n  Second.\n-> END\n",
    ] {
        let directory = project(source).unwrap();
        let mut writer = open(&directory).unwrap();
        let selection = writer.session.model().view().clone();
        let names = labels(&mut writer).unwrap();
        let header = names
            .iter()
            .find(|name| name.to_lowercase().contains("start · start"))
            .unwrap();
        click(&mut writer, &[header]).unwrap();
        assert_eq!(writer.session.model().view(), &selection);
        assert_eq!(writer.session.model().document().source(), source);
        assert!(writer.status.contains("your editor"));
    }
}

#[test]
fn ten_thousand_line_reading_is_paged_and_reused_while_typing() {
    let mut source = ":: start default\n".to_owned();
    for index in 0..10_000 {
        writeln!(&mut source, "> line_{index}@{index:020}\n  Line {index}.").unwrap();
    }
    source.push_str("-> END\n");
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    click(&mut writer, &["Project", "Read beat"]).unwrap();
    let rows = writer.reading.rows.clone();
    let source = writer.reading.source.clone().unwrap();
    assert_eq!(rows.len(), 10_001);
    let mounted = labels(&mut writer).unwrap();
    assert_eq!(
        mounted
            .iter()
            .filter(|name| name.starts_with("Edit passage "))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
    click(&mut writer, &["Edit passage 1"]).unwrap();
    for index in 0..20 {
        writer.change(|model| {
            model.set_draft(format!("Typed {index}"));
            Ok(())
        });
        assert!(Arc::ptr_eq(&rows, &writer.reading.rows));
        assert!(Arc::ptr_eq(
            &source,
            writer.reading.source.as_ref().unwrap()
        ));
    }
    click(
        &mut writer,
        &[
            "Project",
            "Discard draft",
            "Project",
            "Read beat",
            "Next beat entries",
        ],
    )
    .unwrap();
    assert!(Arc::ptr_eq(&rows, &writer.reading.rows));
    let mounted = labels(&mut writer).unwrap();
    assert!(mounted.iter().any(|name| name == "Edit passage 17"));
    assert!(!mounted.iter().any(|name| name == "Edit passage 1"));
    assert_eq!(
        mounted
            .iter()
            .filter(|name| name.starts_with("Edit passage "))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
}

#[test]
fn paged_conditional_entries_keep_their_context_and_plural_fragments_stay_read_only() {
    let mut source = ":: start default\n:if ready()\n".to_owned();
    for index in 0..20 {
        writeln!(
            &mut source,
            "  > line_{index}@{index:020}\n    Line {index}."
        )
        .unwrap();
    }
    source.push_str(":else\n  > plural@aaaaaaaaaaaaaaaaaaaa bind=(count:int=$count)\n    One letter.\n    | Many letters.\n-> END\n");
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    click(&mut writer, &["Project", "Read beat", "Next beat entries"]).unwrap();
    let names = labels(&mut writer).unwrap();
    assert!(names.iter().any(|name| name == "Within :if ready()"));
    assert!(names.iter().any(|name| name == "Within Otherwise"));
    assert!(
        names
            .iter()
            .any(|name| name.contains("Source fragment (read only) · Many letters."))
    );
    assert!(!names.iter().any(|name| name == "Manuscript"));
    assert_eq!(writer.session.model().document().source(), source);
    click(&mut writer, &["Previous beat entries"]).unwrap();
    assert_eq!(writer.reading.page, 0);
}

#[test]
fn preserved_source_selection_distinguishes_readable_script_and_repair() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| model.select(View::Source));
    let names = labels(&mut writer).unwrap();
    assert!(
        names
            .iter()
            .any(|name| name.starts_with("Choose a beat in the outline"))
    );
    assert!(
        !names
            .iter()
            .any(|name| name == "Source editing in your editor")
    );
    drop(writer);
    let directory = project(":: start default\n->\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| model.select(View::Block("start".into())));
    assert!(writer.reading.error.is_some());
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|name| name == "Source editing in your editor")
    );
}

#[path = "inline/tests.rs"]
mod inline;
