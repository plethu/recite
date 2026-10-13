use recite_compiler::authoring::{
    BuildGeneration, BuildInput, BuildInputAuthority, BuildInputKind, BuildInputPolicy,
    BuildRequest, BuildRequestError, SnapshotGeneration,
};
use recite_core::DocumentKey;

// Preserve the former canonicalisation as an oracle for ordering, overlay
// precedence and duplicate errors, independent of the faster sorted pass.
fn previous_canonicalisation(
    mut inputs: Vec<BuildInput>,
) -> Result<Vec<BuildInput>, BuildRequestError> {
    inputs.sort_by(|left, right| {
        left.key()
            .cmp(right.key())
            .then(left.kind().cmp(right.kind()))
            .then(left.authority().cmp(&right.authority()))
    });
    let mut effective: Vec<BuildInput> = Vec::new();
    for input in inputs {
        if let Some(previous) = effective
            .iter_mut()
            .find(|previous| previous.key() == input.key() && previous.kind() == input.kind())
        {
            if input.authority() == BuildInputAuthority::Overlay
                && previous.authority() == BuildInputAuthority::Saved
            {
                *previous = input;
            } else {
                return Err(BuildRequestError::DuplicateInput {
                    key: input.key().clone(),
                    kind: input.kind().clone(),
                });
            }
        } else {
            effective.push(input);
        }
    }
    Ok(effective)
}

#[test]
fn sorted_pass_preserves_overlay_precedence_duplicate_errors_and_source_order()
-> Result<(), Box<dyn std::error::Error>> {
    // Exhaust every sequence up to four inputs over two keys, two kinds and
    // both authorities, including saved/overlay pairs and repeated duplicates.
    for length in 0..=4 {
        for case in 0..8_usize.pow(length) {
            let mut inputs = Vec::new();
            for index in 0..length {
                let code = (case / 8_usize.pow(index)) % 8;
                inputs.push(BuildInput::new(
                    DocumentKey::new(format!("{}.recite", code & 1))?,
                    if code & 2 != 0 {
                        BuildInputKind::Other("metadata".into())
                    } else {
                        BuildInputKind::Source
                    },
                    if code & 4 != 0 {
                        BuildInputAuthority::Overlay
                    } else {
                        BuildInputAuthority::Saved
                    },
                    format!("input {index}"),
                ));
            }
            let expected = previous_canonicalisation(inputs.clone());
            let actual = BuildRequest::new_with_policy(
                BuildGeneration::initial(),
                SnapshotGeneration::initial(),
                inputs,
                BuildInputPolicy::SavedAndOverlays,
            );
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => assert_eq!(actual.inputs(), expected.as_slice()),
                (Err(expected), Err(actual)) => assert_eq!(actual, expected),
                (expected, actual) => panic!("{expected:?} differs from {actual:?}"),
            }
        }
    }
    Ok(())
}

#[test]
fn request_clones_share_large_inputs_and_fingerprints_and_outlive_the_original()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "Unicode café source ".repeat(10000);
    let make = || {
        BuildRequest::new(
            BuildGeneration::new(3),
            SnapshotGeneration::new(7),
            [BuildInput::saved_source(
                DocumentKey::new("a.recite")?,
                &source,
            )],
        )
        .map_err(Box::<dyn std::error::Error>::from)
    };
    let original = make()?;
    let copy = original.clone();
    assert!(std::ptr::eq(
        original.inputs().as_ptr(),
        copy.inputs().as_ptr()
    ));
    assert!(std::ptr::eq(
        original.fingerprints().inputs().as_ptr(),
        copy.fingerprints().inputs().as_ptr()
    ));
    let independent = make()?;
    assert_eq!(copy, independent);
    drop(original);
    assert_eq!(copy.inputs()[0].content(), Some(source.as_str()));
    assert_eq!(copy.fingerprints(), independent.fingerprints());
    let changed = BuildRequest::new(
        BuildGeneration::new(3),
        SnapshotGeneration::new(7),
        [BuildInput::saved_source(
            DocumentKey::new("a.recite")?,
            "changed",
        )],
    )?;
    assert_ne!(copy, changed);
    Ok(())
}
