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
