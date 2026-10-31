use super::*;

#[test]
fn no_op_stable_id_plans_preserve_project_validation_errors() {
    let frozen = ":: frozen default\n> @ez1fh09\n  Frozen.\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(key("frozen.recite"), frozen)],
            [],
        ))
        .unwrap();
    assert!(matches!(
        kernel
            .snapshot()
            .plan_insert_missing_ids(&mut recite_core::StableIdGenerator::seeded(1)),
        Err(AuthoringEditError::NoEdits)
    ));
    for (other, expected) in [
        (":: repeated\n:: repeated\n", "ambiguous"),
        (":: other\n> @Bad\n  Malformed.\n", "unsupported"),
    ] {
        kernel
            .apply(AuthoringRequest::new(
                kernel.snapshot().generation(),
                [
                    SavedDocument::new(key("frozen.recite"), frozen),
                    SavedDocument::new(key("other.recite"), other),
                ],
                [],
            ))
            .unwrap();
        let result = kernel.snapshot().plan_insert_missing_ids_for_document(
            &key("frozen.recite"),
            &mut recite_core::StableIdGenerator::seeded(1),
        );
        match expected {
            "ambiguous" => assert!(matches!(
                result,
                Err(AuthoringEditError::AmbiguousBlock { .. })
            )),
            _ => assert!(matches!(
                result,
                Err(AuthoringEditError::UnsupportedStableId { .. })
            )),
        }
    }
}
