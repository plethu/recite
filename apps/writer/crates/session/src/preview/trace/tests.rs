use super::*;

#[test]
fn long_utf8_values_use_direct_nonempty_windows_without_loss_or_unbounded_output() {
    for text in [
        String::new(),
        format!("{}🚉", "x".repeat(999)),
        "駅🚉".repeat(30_000),
    ] {
        let field = Field::text("String argument", text.as_str());
        let parts = field.part(0).parts;
        let mut restored = String::new();
        for index in 0..parts {
            let part = field.part(index);
            assert!(part.value.len() <= 1_003);
            assert!(text.is_empty() || !part.value.is_empty());
            restored.push_str(&part.value);
        }
        assert_eq!(restored, text);
        assert_eq!(field.part(usize::MAX).part, parts - 1);
    }
}

#[test]
fn large_condition_and_nested_runtime_error_reasons_stay_borrowed_and_paged() {
    use recite_runtime::{DialogueError, preview::PreviewError};
    use recite_writer_model::Workbench;
    let mut model =
        Workbench::new(":: start default\n:if accepts()\n  > @ffee6p8\n    Accepted.\n-> END\n")
            .unwrap();
    model.start_preview().unwrap();
    let request_id = model
        .preview_page()
        .unwrap()
        .condition
        .as_ref()
        .unwrap()
        .id();
    let reason = "駅🚉".repeat(100_000);
    for error in [
        PreviewError::ConditionFailed {
            request_id,
            reason: reason.clone(),
        },
        PreviewError::Runtime(DialogueError::LocaleLookupFailed {
            id: "ffee6p8".into(),
            reason: reason.clone(),
        }),
    ] {
        let fields = error::fields(&error);
        assert!(fields.len() <= 7);
        let field = fields.iter().find(|field| field.name == "Reason").unwrap();
        assert!(
            matches!(field.value, Value::Text(Cow::Borrowed(_))),
            "error text must not be copied before selecting its visible part"
        );
        let parts = field.part(0).parts;
        for index in [0, parts / 2, parts - 1] {
            let part = field.part(index);
            assert!(!part.value.is_empty() && part.value.len() <= 1_003);
            assert!(reason.contains(&part.value));
        }
    }
}

#[test]
fn schema_fingerprint_differences_remain_inspectable_beyond_the_first_hex_page() {
    use recite_runtime::{
        DialogueError,
        preview::PreviewError,
        snapshot::{DialogueContentFingerprintSnapshot, DialogueSchemaFingerprintSnapshot},
    };
    for length in [32, 1_000_026] {
        let mut digest = vec![0; length];
        digest[length - 1] = 255;
        let error = PreviewError::Runtime(DialogueError::SchemaMismatch {
            asset_id: "fixture".into(),
            expected_schema_fingerprint: DialogueSchemaFingerprintSnapshot::Fingerprint(
                DialogueContentFingerprintSnapshot {
                    algorithm: "sha256".into(),
                    digest: vec![0; length],
                },
            ),
            actual_schema_fingerprint: DialogueSchemaFingerprintSnapshot::Fingerprint(
                DialogueContentFingerprintSnapshot {
                    algorithm: "sha256".into(),
                    digest,
                },
            ),
        });
        let fields = error::fields(&error);
        assert_eq!(fields.len(), 6);
        let expected = fields
            .iter()
            .find(|field| field.name == "Expected schema digest")
            .unwrap();
        let actual = fields
            .iter()
            .find(|field| field.name == "Actual schema digest")
            .unwrap();
        assert!(matches!(actual.value, Value::Hex(bytes) if bytes.len() == length));
        let (excerpt, truncated) = actual.excerpt();
        assert!(excerpt.len() <= 160);
        assert_eq!(truncated, length > 80);
        let last = actual.part(usize::MAX);
        assert!(last.value.len() <= 1_000);
        assert!(last.value.ends_with("ff"));
        assert!(expected.part(usize::MAX).value.ends_with("00"));
        if length > 500 {
            assert_eq!(expected.part(0).value, actual.part(0).value);
            assert!(last.part > 0);
        }
    }
    let error = PreviewError::Runtime(DialogueError::SchemaMismatch {
        asset_id: "fixture".into(),
        expected_schema_fingerprint: DialogueSchemaFingerprintSnapshot::NoSchema,
        actual_schema_fingerprint: DialogueSchemaFingerprintSnapshot::NoSchema,
    });
    let fields = error::fields(&error);
    assert_eq!(fields.len(), 4);
    assert_eq!(fields[2].part(0).value, "No schema");
}

#[test]
fn preview_level_errors_name_their_category_and_keep_reasons_borrowed() {
    use recite_runtime::preview::PreviewError;
    fn text<'a>(field: &'a Field<'_>) -> &'a str {
        match &field.value {
            Value::Text(text) => text,
            Value::Hex(_) => panic!("{} should be text", field.name),
        }
    }
    let reason = "missing asset hash";
    // Each variant with the detail it must show; supplied reasons stay borrowed.
    let cases = [
        (
            PreviewError::AssetRevisionFailed {
                reason: reason.into(),
            },
            Some(reason),
        ),
        (PreviewError::ConditionPending, None),
        (PreviewError::ConditionNotPending, None),
        (
            PreviewError::ConditionReplayMismatch {
                mismatch: reason.into(),
            },
            Some(reason),
        ),
        (PreviewError::ConditionRequestIdOverflow, None),
        (PreviewError::SnapshotPendingCondition, None),
        (
            PreviewError::SnapshotEncodeFailed {
                reason: reason.into(),
            },
            Some(reason),
        ),
        (
            PreviewError::SnapshotDecodeFailed {
                reason: reason.into(),
            },
            Some(reason),
        ),
        (PreviewError::SnapshotStateMismatch, None),
    ];
    let mut categories = std::collections::BTreeSet::new();
    for (error, supplied) in &cases {
        let fields = error::fields(error);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "Error");
        assert!(
            categories.insert(text(&fields[0]).to_owned()),
            "duplicate category"
        );
        assert_eq!(fields[1].name, "Reason");
        assert!(!text(&fields[1]).is_empty());
        if let Some(supplied) = supplied {
            assert!(matches!(fields[1].value, Value::Text(Cow::Borrowed(_))));
            assert_eq!(text(&fields[1]), *supplied);
        }
    }
    let format = error::fields(&PreviewError::UnsupportedSnapshotFormat {
        snapshot_format_version: 9,
    });
    assert_eq!(text(&format[0]), "Unsupported snapshot format");
    assert_eq!(
        (format[1].name.as_ref(), text(&format[1])),
        ("Format version", "9")
    );
}
