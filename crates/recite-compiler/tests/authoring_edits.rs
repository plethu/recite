#![cfg(test)]

use std::collections::BTreeMap;

use recite_compiler::{
    AuthoringEditError, AuthoringEditPlan, AuthoringKernel, AuthoringRequest, DocumentVersion,
    OpenDocument, SavedDocument, SnapshotGeneration, SourceEdit, SourceRange,
};
use recite_core::{DocumentKey, SourcePosition};

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test document key is valid")
}

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("test source position is valid")
}

fn apply_plan(
    plan: &AuthoringEditPlan,
    sources: &BTreeMap<DocumentKey, String>,
) -> BTreeMap<DocumentKey, String> {
    let mut edits = BTreeMap::<DocumentKey, Vec<&SourceEdit>>::new();
    for edit in plan.edits() {
        edits.entry(edit.document().clone()).or_default().push(edit);
    }
    let mut result = sources.clone();
    for (document, mut document_edits) in edits {
        let source = result.get(&document).expect("plan document has source");
        document_edits.sort_by_key(|edit| edit.range());
        let mut updated = source.clone();
        for edit in document_edits.into_iter().rev() {
            let (start, end) = offsets(source, edit.range());
            updated.replace_range(start..end, edit.replacement());
        }
        result.insert(document, updated);
    }
    result
}

fn offsets(source: &str, range: SourceRange) -> (usize, usize) {
    (offset(source, range.start()), offset(source, range.end()))
}

fn offset(source: &str, position: SourcePosition) -> usize {
    let mut line = 1;
    let mut line_start = 0;
    for (index, byte) in source.bytes().enumerate() {
        if byte != b'\n' {
            continue;
        }
        let line_end = index.saturating_sub(usize::from(
            index > line_start && source.as_bytes()[index - 1] == b'\r',
        ));
        if line == position.line() {
            return line_start + scalar_offset(&source[line_start..line_end], position);
        }
        line_start = index + 1;
        line += 1;
    }
    if line == position.line() {
        line_start + scalar_offset(&source[line_start..], position)
    } else {
        panic!("position is outside source: {position:?}")
    }
}

fn scalar_offset(line: &str, position: SourcePosition) -> usize {
    let scalar = usize::try_from(position.column() - 1).expect("position fits usize");
    line.char_indices()
        .nth(scalar)
        .map_or(line.len(), |(index, _)| index)
}

fn saved_sources(entries: &[(&str, &str)]) -> BTreeMap<DocumentKey, String> {
    entries
        .iter()
        .map(|(path, source)| (key(path), (*source).to_owned()))
        .collect()
}

#[test]
fn stable_id_plans_are_deterministic_and_preserve_source_bytes() {
    let main = concat!(
        ":: start\r\n",
        "# keep this comment: 💬\r\n",
        "> speaker=é\r\n",
        "  Keep this prose.\r\n",
        "?\r\n",
        "  Keep this choice.\r\n",
    );
    let other = ":: other\r\n> existing@0123456789abcdef0123\r\n  Existing.\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("other.recite"), other)],
            [OpenDocument::new(
                key("main.recite"),
                DocumentVersion::new(7),
                main,
            )],
        ))
        .expect("source accepted");

    let first = kernel
        .snapshot()
        .plan_insert_missing_ids()
        .expect("missing IDs are planable");
    let repeated = kernel
        .snapshot()
        .plan_insert_missing_ids()
        .expect("repeated plan is planable");
    assert_eq!(first, repeated);
    assert_eq!(first.preconditions().len(), 2);
    assert_eq!(first.edits().len(), 2);
    assert_eq!(first.edits()[0].document(), &key("main.recite"));
    assert_eq!(
        first.preconditions()[0].expected_version(),
        Some(DocumentVersion::new(7))
    );
    first
        .validate(kernel.snapshot())
        .expect("fresh plan validates");

    let applied = apply_plan(
        &first,
        &saved_sources(&[("main.recite", main), ("other.recite", other)]),
    );
    let rewritten = applied.get(&key("main.recite")).expect("rewritten main");
    assert!(rewritten.contains("# keep this comment: 💬\r\n"));
    assert!(rewritten.contains("  Keep this prose.\r\n"));
    assert!(rewritten.contains("  Keep this choice.\r\n"));
    let lowered = recite_parser::parse("main.recite", rewritten).lower_source_file();
    assert!(
        lowered.diagnostics.is_empty(),
        "rewritten source: {:?}",
        lowered.diagnostics
    );
    let stable_ids = kernel
        .snapshot()
        .document(&key("main.recite"))
        .expect("main document")
        .summary()
        .stable_ids();
    assert_eq!(stable_ids.len(), 2);
    assert!(
        stable_ids
            .iter()
            .all(|stable| { matches!(stable.source_id(), recite_core::SourceId::Missing) })
    );
    let rewritten_ids = lowered
        .source_file
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| match statement {
            recite_core::Statement::Line(line) => Some(&line.source_id),
            recite_core::Statement::Choice(choice) => Some(&choice.source_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(rewritten_ids.len(), 2);
    assert!(
        rewritten_ids
            .iter()
            .all(|source_id| matches!(source_id, recite_core::SourceId::Frozen { .. }))
    );
}

#[test]
fn stable_id_batch_and_range_plans_scope_edits_but_guard_project_state() {
    let main = concat!(
        ":: start\n",
        ">\n",
        "  Missing line.\n",
        "?\n",
        "  Missing choice.\n",
    );
    let other = ":: other\n>\n  Other missing line.\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("main.recite"), main),
                SavedDocument::new(key("other.recite"), other),
            ],
            [],
        ))
        .expect("source accepted");

    let batch = kernel
        .snapshot()
        .plan_insert_missing_ids_for_document(&key("main.recite"))
        .expect("document batch is planable");
    assert_eq!(batch.preconditions().len(), 2);
    assert_eq!(batch.edits().len(), 2);
    assert!(
        batch
            .edits()
            .iter()
            .all(|edit| edit.document() == &key("main.recite"))
    );

    let range = SourceRange::new(position(2, 1), position(3, 1));
    let ranged = kernel
        .snapshot()
        .plan_insert_missing_ids_in_range(&key("main.recite"), range)
        .expect("range batch is planable");
    assert_eq!(ranged.preconditions().len(), 2);
    assert_eq!(ranged.edits().len(), 1);
    assert_eq!(ranged.edits()[0].document(), &key("main.recite"));
}

#[test]
fn rename_plan_edits_only_definition_and_resolved_references() {
    let main = concat!(
        ":: start\r\n",
        "# unchanged prose\r\n",
        "-> target\r\n",
        "-> main.recite::target\r\n",
        ":: target\r\n",
        "> line@11111111111111111111\r\n",
        "  unchanged dialogue\r\n",
    );
    let other = ":: other\r\n-> main.recite::target\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("main.recite"), main),
                SavedDocument::new(key("other.recite"), other),
            ],
            [],
        ))
        .expect("source accepted");
    let plan = kernel
        .snapshot()
        .plan_rename_block(&key("main.recite"), position(5, 4), "renamed")
        .expect("unique block rename is planable");
    assert_eq!(plan.edits().len(), 4);
    assert_eq!(plan.preconditions().len(), 2);
    let applied = apply_plan(
        &plan,
        &saved_sources(&[("main.recite", main), ("other.recite", other)]),
    );
    let rewritten_main = applied.get(&key("main.recite")).expect("rewritten main");
    let rewritten_other = applied.get(&key("other.recite")).expect("rewritten other");
    assert!(rewritten_main.contains("# unchanged prose\r\n"));
    assert!(rewritten_main.contains("  unchanged dialogue\r\n"));
    assert!(rewritten_main.contains(":: renamed\r\n"));
    assert!(rewritten_main.contains("-> renamed\r\n"));
    assert!(rewritten_main.contains("-> main.recite::renamed\r\n"));
    assert!(rewritten_other.contains("-> main.recite::renamed\r\n"));
    assert!(rewritten_main.contains("line@11111111111111111111"));
    assert!(
        recite_parser::parse("main.recite", rewritten_main)
            .lower_source_file()
            .diagnostics
            .is_empty()
    );
    assert!(
        recite_parser::parse("other.recite", rewritten_other)
            .lower_source_file()
            .diagnostics
            .is_empty()
    );
}

#[test]
fn block_stub_plan_preserves_target_crlf_and_reports_provenance() {
    let source = ":: start\n-> target.recite::missing\n";
    let target = "# target comment\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("main.recite"), source),
                SavedDocument::new(key("target.recite"), target),
            ],
            [],
        ))
        .expect("source accepted");
    let plan = kernel
        .snapshot()
        .plan_create_block_stub(&key("main.recite"), position(2, 23))
        .expect("missing qualified target is planable");
    assert_eq!(plan.edits().len(), 1);
    assert_eq!(plan.edits()[0].document(), &key("target.recite"));
    assert!(matches!(
        plan.operation(),
        recite_compiler::AuthoringEditOperation::CreateBlockStub {
            source,
            target,
            block,
            ..
        } if source == &key("main.recite") && target == &key("target.recite") && block.as_str() == "missing"
    ));
    let applied = apply_plan(
        &plan,
        &saved_sources(&[("main.recite", source), ("target.recite", target)]),
    );
    let Some(rewritten_target) = applied.get(&key("target.recite")) else {
        panic!("rewritten target is missing");
    };
    assert_eq!(rewritten_target, "# target comment\r\n:: missing\r\n");
    let Some(rewritten_target) = applied.get(&key("target.recite")) else {
        panic!("rewritten target is missing");
    };
    assert!(
        recite_parser::parse("target.recite", rewritten_target)
            .lower_source_file()
            .diagnostics
            .is_empty()
    );
}

#[test]
fn edit_plans_refuse_stale_generation_collisions_and_partial_recovery() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: target\n-> target\n",
            )],
            [],
        ))
        .expect("source accepted");
    let plan = kernel
        .snapshot()
        .plan_rename_block(&key("main.recite"), position(1, 4), "renamed")
        .expect("rename is planable");
    kernel
        .apply(AuthoringRequest::new(
            plan.expected_generation(),
            [],
            [OpenDocument::new(
                key("main.recite"),
                DocumentVersion::new(1),
                ":: target\n-> target\n",
            )],
        ))
        .expect("overlay accepted");
    assert!(matches!(
        plan.validate(kernel.snapshot()),
        Err(AuthoringEditError::StaleGeneration { .. })
    ));

    let mut collision = AuthoringKernel::new();
    collision
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: target\n:: renamed\n",
            )],
            [],
        ))
        .expect("collision source accepted");
    assert!(matches!(
        collision
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::DestinationCollision { .. })
    ));

    let mut partial = AuthoringKernel::new();
    partial
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("clean.recite"), ":: clean\n>\n  Missing.\n"),
                SavedDocument::new(key("broken.recite"), ":: broken\n> line@bad\n  Broken.\n"),
            ],
            [],
        ))
        .expect("recoverable source accepted");
    assert!(matches!(
        partial.snapshot().plan_insert_missing_ids(),
        Err(AuthoringEditError::UnsupportedStableId { .. })
    ));
}
