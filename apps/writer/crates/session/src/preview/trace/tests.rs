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
    let mut model = Workbench::new(
        ":: start default\n:if accepts()\n  > line@11111111111111111111\n    Accepted.\n-> END\n",
    )
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
            id: "11111111111111111111".into(),
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
