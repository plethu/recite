use std::cell::Cell;

use crate::authoring::{
    AuthoringKernel, AuthoringRequest, Interrupted, SavedDocument, WorkControl,
};
use recite_core::DocumentKey;

fn request(kernel: &AuthoringKernel, source: &str) -> AuthoringRequest {
    AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("a.recite").unwrap(),
            source,
        )],
        [],
    )
}

#[test]
fn invalid_cached_coordinates_fall_back_to_cold_analysis() {
    use super::EffectiveDocument;
    use crate::authoring::{CancellationToken, DocumentLayer};
    use std::sync::Arc;

    let key = DocumentKey::new("a.recite").unwrap();
    let text: Arc<str> = Arc::from(":: a default\n-> END\n:: b\n-> missing\n");
    let control = CancellationToken::new();
    let document = EffectiveDocument {
        key: &key,
        text: &text,
        layer: DocumentLayer::Saved,
        version: None,
    };
    let mut old = super::analyze(&document, None, None, &control).unwrap();
    let ranges = old.summary.region_ranges();
    Arc::make_mut(&mut old.summary).relocate_region(&ranges, |span| {
        span.start = recite_core::SourcePosition::new(u32::MAX, 1).unwrap();
        span.end = None;
    });
    let edited: Arc<str> = Arc::from(format!("\n{text}"));
    let next = EffectiveDocument {
        text: &edited,
        ..document
    };
    let warm = super::analyze(&next, Some(&old), None, &control).unwrap();
    let cold = super::analyze(&next, None, None, &control).unwrap();
    assert_eq!(warm.summary, cold.summary);
    assert_eq!(warm.project_facts, cold.project_facts);
    assert_eq!(warm.parse_diagnostics, cold.parse_diagnostics);
    assert_eq!(warm.local_diagnostics, cold.local_diagnostics);
}

#[test]
fn typing_and_line_insertion_parse_only_the_changed_region() {
    let source = ":: a default\n> a@11111111111111111111\n  Hello.\n:: b\n> b@22222222222222222222\n  Later.\n:: c\n-> END\n";
    let mut kernel = AuthoringKernel::new();
    kernel.apply(request(&kernel, source)).unwrap();
    for replacement in ["New prose.", "New\n  paragraph.", "Hello."] {
        super::PARSED_REGIONS.with(|count| count.set(0));
        kernel
            .apply(request(&kernel, &source.replace("Hello.", replacement)))
            .unwrap();
        assert_eq!(super::PARSED_REGIONS.with(Cell::get), 1);
    }
    super::PARSED_REGIONS.with(|count| count.set(0));
    kernel
        .apply(request(
            &kernel,
            &source.replace("Hello.", "Hello.\nbroken"),
        ))
        .unwrap();
    assert_eq!(
        super::PARSED_REGIONS.with(Cell::get),
        3,
        "file-wide recovery changes require local revalidation"
    );
}

struct Budget(Cell<usize>);
impl WorkControl for Budget {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.0.set(self.0.get().checked_sub(1).ok_or(Interrupted)?);
        Ok(())
    }
}

#[test]
fn interrupted_region_candidates_leave_committed_analysis_unchanged() {
    let source: String = (0..20)
        .map(|n| format!(":: block{n}\n> line@{n:020x}\n  Hello.\n"))
        .collect();
    let mut kernel = AuthoringKernel::new();
    kernel.apply(request(&kernel, &source)).unwrap();
    let snapshot = kernel.snapshot().clone();
    let edited = source.replace("Hello.", "New\n  paragraph.");
    let mut completed = false;
    for allowance in 0..300 {
        let result =
            kernel.apply_with_control(request(&kernel, &edited), &Budget(Cell::new(allowance)));
        if result.is_ok() {
            completed = true;
            break;
        }
        assert_eq!(kernel.snapshot().generation(), snapshot.generation());
        assert_eq!(kernel.snapshot().documents(), snapshot.documents());
    }
    assert!(completed, "bounded candidate eventually completes");
    let mut cold = AuthoringKernel::new();
    cold.apply(request(&cold, &edited)).unwrap();
    for (warm, cold) in kernel
        .snapshot()
        .documents()
        .iter()
        .zip(cold.snapshot().documents())
    {
        assert_eq!(warm.summary(), cold.summary());
        assert_eq!(warm.diagnostics(), cold.diagnostics());
    }
}

#[test]
fn recovery_revalidates_echo_consumers_without_visiting_unrelated_files() {
    use crate::validation::incremental::VALIDATED_DOCUMENTS;
    let mut saved: Vec<_> = (0..30)
        .map(|n| {
            SavedDocument::new(
                DocumentKey::new(format!("file{n:02}.recite")).unwrap(),
                format!(
                    ":: block{n}{}\n> line@{n:020x}\n  Hello.\n",
                    if n == 0 { " default" } else { "" }
                ),
            )
        })
        .collect();
    let original = saved[0].text().to_owned();
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            saved.clone(),
            [],
        ))
        .unwrap();
    for source in [format!("{original}stray text\n"), original] {
        VALIDATED_DOCUMENTS.with(|count| count.set(0));
        saved[0] = SavedDocument::new(saved[0].key().clone(), source);
        kernel
            .apply(AuthoringRequest::new(
                kernel.snapshot().generation(),
                saved.clone(),
                [],
            ))
            .unwrap();
        assert_eq!(VALIDATED_DOCUMENTS.with(Cell::get), 1);
    }
}
