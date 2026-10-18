use crate::authoring::{
    AuthoringError, AuthoringKernel, AuthoringRequest, CancellationToken, Interrupted,
    SavedDocument, WorkControl,
};
use crate::validation::incremental::VALIDATED_DOCUMENTS;
use recite_core::DocumentKey;
use std::cell::Cell;
use std::{collections::BTreeSet, sync::Arc};

struct Budget(Cell<usize>);
impl WorkControl for Budget {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        let next = self.0.get().checked_sub(1).ok_or(Interrupted)?;
        self.0.set(next);
        Ok(())
    }
}

fn request(kernel: &AuthoringKernel, source: &str) -> AuthoringRequest {
    AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("large.recite").unwrap(),
            source,
        )],
        [],
    )
}

#[test]
fn reference_edits_do_not_repeat_ten_thousand_unchanged_identity_checks() {
    let mut source = ":: start default\n".to_owned();
    for id in 0..10_000 {
        source.push_str(&format!("> line@{id:020x}\n  Hello.\n"));
    }
    source.push_str("-> missing\n");
    let mut kernel = AuthoringKernel::new();
    VALIDATED_DOCUMENTS.with(|count| count.set(0));
    kernel.apply(request(&kernel, &source)).unwrap();
    assert!(VALIDATED_DOCUMENTS.with(Cell::get) > 0);
    assert_eq!(kernel.snapshot().diagnostics().len(), 1);
    for target in ["start", "absent", "END", "start"] {
        VALIDATED_DOCUMENTS.with(|count| count.set(0));
        kernel
            .apply(request(
                &kernel,
                &source.replace("-> missing", &format!("-> {target}")),
            ))
            .unwrap();
        assert_eq!(
            VALIDATED_DOCUMENTS.with(Cell::get),
            0,
            "unchanged identities must not enter full project validation"
        );
        assert_eq!(
            kernel.snapshot().diagnostics().len(),
            usize::from(target == "absent")
        );
    }
}

#[test]
fn every_reference_checkpoint_cancels_without_publishing_and_retries_exactly() {
    let source = ":: start default\n> line@11111111111111111111\n  Hello.\n-> missing\n";
    for target in ["start", "absent", "other.recite::missing"] {
        let mut kernel = AuthoringKernel::new();
        kernel.apply(request(&kernel, source)).unwrap();
        let before = kernel.snapshot().clone();
        let request = request(
            &kernel,
            &source.replace("-> missing", &format!("-> {target}")),
        );
        let expected = kernel
            .updated(request.clone(), &CancellationToken::new())
            .unwrap();
        let mut committed = false;
        for cutoff in 0..256 {
            match kernel.apply_with_control(request.clone(), &Budget(Cell::new(cutoff))) {
                Err(AuthoringError::Interrupted(_)) => {
                    assert_eq!(kernel.snapshot(), &before);
                    let retry = kernel
                        .updated(request.clone(), &CancellationToken::new())
                        .unwrap();
                    assert_eq!(retry.snapshot(), expected.snapshot());
                }
                Ok(_) => {
                    committed = true;
                    break;
                }
                Err(error) => panic!("unexpected {error}"),
            }
        }
        assert!(committed);
        assert_eq!(kernel.snapshot(), expected.snapshot());
    }
}

#[test]
fn reference_edit_shares_a_thousand_structural_errors_and_untouched_reports() {
    use super::super::{ProjectDiagnostics, ProjectFacts, ProjectIndex};
    use crate::validation::ValidationInput;

    let facts = |path: &str, source: &str| {
        let parsed = recite_parser::parse(path, source).lower_source_file();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        Arc::new(ProjectFacts::collect(ValidationInput::all_complete(
            &parsed.source_file,
        )))
    };
    let large = DocumentKey::new("large.recite").unwrap();
    let other = DocumentKey::new("other.recite").unwrap();
    let mut source = ":: start default\n".to_owned();
    for _ in 0..1_001 {
        source.push_str("> line@11111111111111111111\n  Duplicate.\n");
    }
    source.push_str("-> missing\n");
    let sibling = facts(
        other.as_str(),
        ":: sibling\n> line@22222222222222222222\n  First.\n> line@22222222222222222222\n  Duplicate.\n-> missing\n",
    );
    let control = CancellationToken::new();
    let mut index = ProjectIndex::default();
    let mut reports = ProjectDiagnostics::default();
    index
        .update(
            [
                (large.clone(), Some(facts(large.as_str(), &source))),
                (other.clone(), Some(Arc::clone(&sibling))),
            ],
            true,
            &mut reports,
            &control,
        )
        .unwrap();
    assert_eq!(reports[&large].structural.len(), 1_000);
    assert_eq!(reports[&large].references.len(), 1);
    assert_eq!(reports[&other].structural.len(), 1);
    assert_eq!(reports[&other].references.len(), 1);
    let before = Arc::new(reports);
    let retained_values = before.as_ref().clone();
    let mut current = Arc::clone(&before);
    let next = facts(large.as_str(), &source.replace("-> missing", "-> END"));
    let affected = index
        .update(
            [(large.clone(), Some(Arc::clone(&next)))],
            true,
            Arc::make_mut(&mut current),
            &control,
        )
        .unwrap();
    assert_eq!(affected, BTreeSet::from([large.clone()]));
    assert!(Arc::ptr_eq(
        &current[&large].structural,
        &before[&large].structural
    ));
    assert!(current[&large].references.is_empty());
    assert_eq!(before[&large].references.len(), 1);
    assert!(Arc::ptr_eq(
        &current[&other].structural,
        &before[&other].structural
    ));
    assert!(Arc::ptr_eq(
        &current[&other].references,
        &before[&other].references
    ));
    assert_eq!(before.as_ref(), &retained_values);

    let mut cold = ProjectDiagnostics::default();
    ProjectIndex::default()
        .update(
            [(large, Some(next)), (other, Some(sibling))],
            true,
            &mut cold,
            &control,
        )
        .unwrap();
    assert_eq!(current.as_ref(), &cold);
}
