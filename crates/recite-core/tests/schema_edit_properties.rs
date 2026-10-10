use proptest::prelude::*;
use recite_core::schema::{
    ConditionDefinition, ConditionReturnType, SchemaDeclarationKind, SchemaSource,
    SchemaSourceEdit, SchemaSourceEditError, load_schema_source_str,
};

const BASE: &str = "schema_version = 1\n[producer] # owner sentinel\nid = \"dialogue\" # identity sentinel\n[speakers.hero]\ndisplay_name = \"Hero\" # name sentinel\n";

fn load(text: &str) -> Result<SchemaSource, TestCaseError> {
    let report = load_schema_source_str("edit.toml", text);
    prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report
        .source
        .ok_or_else(|| TestCaseError::fail("valid generated schema required"))
}

proptest! {
    #[test]
    fn edit_plans_and_rejected_operations_preserve_source_identity(
        changes in prop::collection::vec((0u8..6, "[A-Za-z界🦀][^\\r\\n]{0,20}"), 1..12),
        crlf in any::<bool>(),
        final_newline in any::<bool>(),
    ) {
        let mut text = BASE.to_owned();
        if !final_newline { text.pop(); }
        if crlf { text = text.replace('\n', "\r\n"); }
        let mut current = load(&text)?;
        let mut producer = "dialogue".to_owned();
        let mut display = Some("Hero".to_owned());
        for (operation, value) in changes {
            let edit = match operation {
                0 => SchemaSourceEdit::SetProducerId(value.clone()),
                1 => SchemaSourceEdit::SetSpeakerDisplayName { name: "hero".to_owned(), display_name: Some(value.clone()) },
                2 => SchemaSourceEdit::SetSpeakerDisplayName { name: "hero".to_owned(), display_name: None },
                3 => SchemaSourceEdit::SetProducerId(" \t".to_owned()),
                4 => SchemaSourceEdit::SetSpeakerDisplayName { name: "hero".to_owned(), display_name: Some(" \t".to_owned()) },
                _ => SchemaSourceEdit::RemoveDeclaration { kind: SchemaDeclarationKind::Speaker, name: "missing".to_owned() },
            };
            let before = current.clone();
            let result = current.apply_edit(edit.clone());
            if operation >= 3 {
                prop_assert!(matches!(result, Err(SchemaSourceEditError::InvalidArgument(_))));
                prop_assert_eq!(&current, &before);
                prop_assert_eq!(current.source_fingerprint(), before.source_fingerprint());
                prop_assert_eq!(current.source_text_fingerprint(), before.source_text_fingerprint());
                continue;
            }
            result.map_err(|error| TestCaseError::fail(error.to_string()))?;
            let plan = before.plan_edit(edit.clone()).map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(plan.edit(), &edit);
            prop_assert_eq!(plan.expected_source_fingerprint(), before.source_fingerprint());
            prop_assert_eq!(plan.expected_text_fingerprint(), &before.source_text_fingerprint());
            let mut planned = before;
            plan.apply(&mut planned).map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(&planned, &current);
            match operation {
                0 => producer = value,
                1 => display = Some(value),
                _ => display = None,
            }
            let actual_producer = current.schema().producer_metadata.as_ref()
                .and_then(|metadata| metadata.producer.as_ref()).map(|identity| identity.id());
            prop_assert_eq!(actual_producer, Some(producer.as_str()));
            prop_assert_eq!(&current.schema().speakers["hero"].display_name, &display);
            let updated = current.source_text();
            prop_assert!(updated.contains("# owner sentinel"));
            prop_assert!(updated.contains("# identity sentinel"));
            prop_assert_eq!(updated.ends_with('\n'), final_newline);
            prop_assert_eq!(updated.contains("\r\n"), crlf);
            prop_assert_eq!(load(&updated)?, current.clone());
        }
    }
}

#[test]
fn removing_each_unused_declaration_leaves_a_valid_reparse()
-> Result<(), Box<dyn std::error::Error>> {
    for (kind, section, declaration) in [
        (
            SchemaDeclarationKind::Type,
            "types",
            "kind = \"enum\"\nvalues = [\"one\"]",
        ),
        (
            SchemaDeclarationKind::Registry,
            "registries",
            "values = [\"one\"]",
        ),
        (
            SchemaDeclarationKind::Speaker,
            "speakers",
            "display_name = \"One\"",
        ),
        (
            SchemaDeclarationKind::Condition,
            "conditions",
            "returns = \"bool\"",
        ),
        (
            SchemaDeclarationKind::AvailabilityReason,
            "availability_reasons",
            "template = \"Blocked\"",
        ),
        (
            SchemaDeclarationKind::Effect,
            "effects",
            "modes = [\"immediate\"]",
        ),
        (
            SchemaDeclarationKind::MetadataDomain,
            "metadata_domains",
            "kind = \"flat\"\nvalues = [\"one\"]",
        ),
        (
            SchemaDeclarationKind::Metadata,
            "metadata",
            "targets = [\"choice\"]\ntype = \"string\"",
        ),
        (
            SchemaDeclarationKind::ProjectionQuery,
            "projection_queries",
            "returns = \"int\"\nmax_calls_per_event = 1",
        ),
        (
            SchemaDeclarationKind::PresentationProjector,
            "presentation_projectors",
            "candidates = { kind = \"runtime_event\", event = \"dialogue\" }",
        ),
        (
            SchemaDeclarationKind::Markup,
            "markup",
            "requires_closing = true\ntranslatable = true\nallows_nesting = false",
        ),
    ] {
        let text = format!(
            "schema_version = 1\n[producer]\nid = \"dialogue\"\n[{section}.one]\n{declaration}\n"
        );
        let report = load_schema_source_str("remove.toml", &text);
        assert!(
            report.diagnostics.is_empty(),
            "{section}: {:?}",
            report.diagnostics
        );
        let mut source = report.source.expect("valid unused declaration");
        source.apply_edit(SchemaSourceEdit::RemoveDeclaration {
            kind,
            name: "one".to_owned(),
        })?;
        let exported: serde_json::Value = serde_json::from_str(&source.export_json())?;
        assert!(exported[section].get("one").is_none(), "{section}");
        let after = source.clone();
        assert!(matches!(
            source.apply_edit(SchemaSourceEdit::RemoveDeclaration {
                kind,
                name: "one".to_owned()
            }),
            Err(SchemaSourceEditError::InvalidArgument(_))
        ));
        assert_eq!(source, after);
        let reloaded = load_schema_source_str("remove.toml", &source.source_text());
        assert!(reloaded.diagnostics.is_empty());
        assert_eq!(reloaded.source, Some(source));
    }
    Ok(())
}

#[test]
fn invalid_names_duplicate_declarations_and_referenced_removal_are_atomic() {
    let text = format!(
        "{BASE}[types.mood]\nkind = \"enum\"\nvalues = [\"calm\"]\n[conditions.ready]\nreturns = \"enum:mood\"\n"
    );
    let mut source = load_schema_source_str("atomic.toml", &text)
        .source
        .expect("valid schema");
    let before = source.clone();
    for edit in [
        SchemaSourceEdit::SetSpeakerDisplayName {
            name: "bad name".to_owned(),
            display_name: Some("A".to_owned()),
        },
        SchemaSourceEdit::RemoveDeclaration {
            kind: SchemaDeclarationKind::Speaker,
            name: "bad name".to_owned(),
        },
        SchemaSourceEdit::RemoveDeclaration {
            kind: SchemaDeclarationKind::Effect,
            name: "missing".to_owned(),
        },
        SchemaSourceEdit::AddCondition {
            name: "ready".to_owned(),
            definition: ConditionDefinition {
                params: Vec::new(),
                returns: ConditionReturnType::Bool,
                availability_reason: None,
            },
        },
        SchemaSourceEdit::AddCondition {
            name: "bad name".to_owned(),
            definition: ConditionDefinition {
                params: Vec::new(),
                returns: ConditionReturnType::Bool,
                availability_reason: None,
            },
        },
    ] {
        assert!(matches!(
            source.apply_edit(edit),
            Err(SchemaSourceEditError::InvalidArgument(_))
        ));
        assert_eq!(source, before);
        assert_eq!(source.source_fingerprint(), before.source_fingerprint());
    }
    let Err(SchemaSourceEditError::Diagnostics(diagnostics)) =
        source.apply_edit(SchemaSourceEdit::RemoveDeclaration {
            kind: SchemaDeclarationKind::Type,
            name: "mood".to_owned(),
        })
    else {
        panic!("removing a referenced type must fail through schema diagnostics");
    };
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.record().is_ok())
    );
    assert_eq!(source, before);
}
