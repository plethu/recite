use super::{MAX_CONTEXT_PASSAGES, MAX_VALIDATED_PASSAGES};
use crate::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument};
use recite_core::DocumentKey;

#[test]
fn independent_large_files_do_not_materialize_one_project_sized_context() {
    let files = (0..17).map(|file| {
        let mut text = format!(
            ":: scene_{file}{}\n",
            if file == 0 { " default" } else { "" }
        );
        for line in 0..1000 {
            text.push_str(&format!("> line@{:020x}\n  Hello.\n", file * 1000 + line));
        }
        SavedDocument::new(
            DocumentKey::new(format!("scene-{file}.recite")).unwrap(),
            text,
        )
    });
    MAX_VALIDATED_PASSAGES.with(|count| count.set(0));
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            files,
            [],
        ))
        .unwrap();
    assert!(
        kernel
            .snapshot()
            .documents()
            .iter()
            .all(|document| document.diagnostics().is_empty())
    );
    let largest = MAX_VALIDATED_PASSAGES.with(std::cell::Cell::get);
    assert!(largest > 0 && largest <= MAX_CONTEXT_PASSAGES);
}

#[test]
fn oversized_shared_provider_is_validated_once_with_bounded_dependent_work() {
    use super::VALIDATED_BATCHES;
    const PASSAGES: usize = 4_096;
    let mut provider = String::from(":: provider default\n");
    for n in 0..PASSAGES {
        provider.push_str(&format!("> line@{n:020x}\n  Provider prose.\n"));
    }
    provider.push_str("-> END\n");
    let mut kernel = AuthoringKernel::new();
    for header in ["provider", "renamed", "provider"] {
        let mut files = vec![SavedDocument::new(
            DocumentKey::new("provider.recite").unwrap(),
            provider.replacen(":: provider", &format!(":: {header}"), 1),
        )];
        for n in 0..128 {
            files.push(SavedDocument::new(
                DocumentKey::new(format!("user-{n:03}.recite")).unwrap(),
                format!(
                    ":: user_{n}\n> line@{:020x}\n  User prose.\n-> provider.recite::provider\n",
                    PASSAGES + n
                ),
            ));
        }
        let lowered = files
            .iter()
            .map(|file| {
                recite_parser::lower_source_file(file.key().as_str(), file.text()).source_file
            })
            .collect::<Vec<_>>();
        let expected = crate::validation::validate_source_files(&lowered).diagnostics;
        VALIDATED_BATCHES.with(|count| count.set(0));
        MAX_VALIDATED_PASSAGES.with(|count| count.set(0));
        kernel
            .apply(AuthoringRequest::new(
                kernel.snapshot().generation(),
                files,
                [],
            ))
            .unwrap();
        let actual = kernel
            .snapshot()
            .diagnostics()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(VALIDATED_BATCHES.with(std::cell::Cell::get), 1);
        let largest = MAX_VALIDATED_PASSAGES.with(std::cell::Cell::get);
        assert!((PASSAGES..=PASSAGES + MAX_CONTEXT_PASSAGES).contains(&largest));
    }
    let saved = kernel
        .snapshot()
        .documents()
        .iter()
        .filter(|file| file.key().as_str() != "provider.recite")
        .map(|file| SavedDocument::new(file.key().clone(), file.source_text()))
        .collect::<Vec<_>>();
    VALIDATED_BATCHES.with(|count| count.set(0));
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved,
            [],
        ))
        .unwrap();
    assert_eq!(VALIDATED_BATCHES.with(std::cell::Cell::get), 1);
    let remaining = kernel
        .snapshot()
        .documents()
        .iter()
        .map(|file| {
            recite_parser::lower_source_file(file.key().as_str(), file.source_text()).source_file
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kernel
            .snapshot()
            .diagnostics()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        crate::validation::validate_source_files(&remaining).diagnostics
    );
}
