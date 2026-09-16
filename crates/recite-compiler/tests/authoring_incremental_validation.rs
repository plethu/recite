#![cfg(test)]

//! Cached authoring diagnostics must equal the uncached batch validator, including locations.
use recite_compiler::{AuthoringKernel, AuthoringRequest, SavedDocument, ValidationInput};
use recite_core::{Diagnostic, DocumentKey, ProjectSchema};

fn ordered(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by(|a, b| {
        a.span
            .file
            .cmp(&b.span.file)
            .then(a.span.start.cmp(&b.span.start))
            .then(a.span.end.cmp(&b.span.end))
            .then(a.code.as_str().cmp(b.code.as_str()))
            .then(a.message.cmp(&b.message))
    });
    diagnostics
}
fn compare(
    kernel: &mut AuthoringKernel,
    documents: &[(&str, &str)],
    schema: Option<&ProjectSchema>,
    complete: bool,
) {
    let saved: Vec<_> = documents
        .iter()
        .map(|(name, text)| SavedDocument::new(DocumentKey::new(*name).unwrap(), *text))
        .collect();
    kernel
        .apply(
            AuthoringRequest::new(kernel.snapshot().generation(), saved, [])
                .with_project_completeness(complete),
        )
        .unwrap();
    let parsed: Vec<_> = documents
        .iter()
        .map(|(name, text)| recite_parser::parse(*name, *text).lower_source_file())
        .collect();
    let inputs: Vec<_> = parsed
        .iter()
        .map(|p| {
            ValidationInput::new(
                &p.source_file,
                kernel
                    .snapshot()
                    .document(&DocumentKey::new(&p.source_file.path).unwrap())
                    .unwrap()
                    .participation(),
            )
        })
        .collect();
    let report = match (schema, complete) {
        (None, true) => recite_compiler::validate_source_files_with_participation(&inputs),
        (None, false) => recite_compiler::validate_source_files_with_incomplete_project(&inputs),
        (Some(schema), true) => {
            recite_compiler::validate_source_files_with_participation_with_schema(&inputs, schema)
        }
        (Some(schema), false) => {
            recite_compiler::validate_source_files_with_incomplete_project_with_schema(
                &inputs, schema,
            )
        }
    };
    let expected = ordered(
        parsed
            .into_iter()
            .flat_map(|p| p.diagnostics)
            .chain(report.diagnostics)
            .collect(),
    );
    let actual = ordered(
        kernel
            .snapshot()
            .documents()
            .iter()
            .flat_map(|d| d.diagnostics().iter().cloned())
            .collect(),
    );
    assert_eq!(
        actual, expected,
        "complete={complete}, sources={documents:?}"
    );
}

#[test]
fn edits_recovery_removal_and_completeness_match_batch_validation() {
    let a = ":: start default\n> hello@11111111111111111111\n  Hello.\n-> b.recite::there\n";
    let b = ":: there\n> next@22222222222222222222\n  Next.\n-> END\n";
    for schema in [None, Some(ProjectSchema::empty_v1())] {
        let mut kernel = schema
            .clone()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema);
        for complete in [true, false, true] {
            for edited in [
                a.to_owned(),
                a.replace("Hello.", "A longer café sentence."),
                a.replace("Hello.", "[unknown]bad markup[/unknown]"),
                a.replace("Hello.", "Hello {unbound}."),
                a.replace("Hello.", "Hello.\n  Another paragraph."),
                a.replace("11111111111111111111", "22222222222222222222"),
                a.replace(":: start default", ":: there default"),
                a.replace(" default", ""),
                a.replace("b.recite::there", "b.recite::missing"),
                a.replace("11111111111111111111", "bad"),
                a.replace("-> b.recite::there", ":if broken("),
                a.to_owned(),
            ] {
                compare(
                    &mut kernel,
                    &[("a.recite", &edited), ("b.recite", b)],
                    schema.as_ref(),
                    complete,
                );
            }
            compare(&mut kernel, &[("a.recite", a)], schema.as_ref(), complete);
            compare(
                &mut kernel,
                &[
                    ("a.recite", a),
                    ("b.recite", &b.replace(":: there", ":: relocated")),
                ],
                schema.as_ref(),
                complete,
            );
            compare(&mut kernel, &[], schema.as_ref(), complete);
        }
    }
}

#[test]
fn every_source_fixture_matches_the_batch_validator_after_a_previous_revision() {
    fn sources(path: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "recite") {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recite"),
        &mut paths,
    );
    paths.sort();
    assert!(!paths.is_empty());
    let mut kernel = AuthoringKernel::with_schema(ProjectSchema::empty_v1());
    let schema = ProjectSchema::empty_v1();
    for path in paths {
        let source = std::fs::read_to_string(path).unwrap();
        compare(
            &mut kernel,
            &[
                ("fixture.recite", &source),
                ("other.recite", ":: other\n-> fixture.recite::start\n"),
            ],
            Some(&schema),
            true,
        );
    }
}

#[test]
fn echo_targets_and_related_diagnostic_locations_are_invalidated() {
    let line = ":: first default\n> spoken@11111111111111111111\n  First.\n-> END\n";
    let choice = ":: second\n? reply@22222222222222222222 echo=line(11111111111111111111)\n  Again.\n  -> END\n";
    let mut kernel = AuthoringKernel::new();
    compare(
        &mut kernel,
        &[("a.recite", line), ("b.recite", choice)],
        None,
        true,
    );
    assert!(
        kernel
            .snapshot()
            .documents()
            .iter()
            .all(|d| d.diagnostics().is_empty())
    );
    compare(
        &mut kernel,
        &[
            (
                "a.recite",
                &line.replace("11111111111111111111", "33333333333333333333"),
            ),
            ("b.recite", choice),
        ],
        None,
        true,
    );
    assert!(
        !kernel
            .snapshot()
            .document(&DocumentKey::new("b.recite").unwrap())
            .unwrap()
            .diagnostics()
            .is_empty()
    );
    compare(
        &mut kernel,
        &[
            ("a.recite", line),
            (
                "b.recite",
                &choice.replace(
                    "echo=line(11111111111111111111)",
                    "echo=line(44444444444444444444)",
                ),
            ),
        ],
        None,
        true,
    );
    let duplicate = line.replace(":: first default", ":: second");
    compare(
        &mut kernel,
        &[("a.recite", line), ("b.recite", &duplicate)],
        None,
        true,
    );
    let before = kernel
        .snapshot()
        .document(&DocumentKey::new("b.recite").unwrap())
        .unwrap()
        .diagnostics()
        .to_vec();
    compare(
        &mut kernel,
        &[("a.recite", &format!("\n{line}")), ("b.recite", &duplicate)],
        None,
        true,
    );
    assert_ne!(
        before,
        kernel
            .snapshot()
            .document(&DocumentKey::new("b.recite").unwrap())
            .unwrap()
            .diagnostics()
    );
}
