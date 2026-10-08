use super::*;
use recite_core::schema::{ParameterDefinition, SchemaTypeRef};

#[test]
fn declaration_browser_preserves_all_families_and_sorts_names() -> Result<(), String> {
    let report = recite_core::schema::load_schema_manifest_str(
        "full_manifest.json",
        include_str!("../../../../../../../fixtures/schema/valid/full_manifest.json"),
    );
    let schema = report
        .schema
        .ok_or_else(|| format!("{:?}", report.diagnostics))?;
    let rows = entries(&schema);
    assert!(
        rows.windows(2)
            .all(|pair| (&pair[0].0, &pair[0].1) <= (&pair[1].0, &pair[1].1))
    );
    for (name, kind, detail) in [
        ("can_open", "Condition", "item: item"),
        ("open", "Effect request", "item: item"),
        ("rhea", "Speaker", "rhea"),
        ("item", "Registry", "brass_key"),
    ] {
        assert!(
            rows.iter()
                .any(|row| row == &(name.into(), kind.into(), detail.into())),
            "{name}: {rows:?}"
        );
    }
    for (kind, names) in [
        ("type", vec!["mood"]),
        ("availability_reason", vec!["missing_key"]),
        ("metadata_domain", vec!["tone", "tone_by_speaker"]),
        ("metadata", vec!["skill", "threshold", "tag"]),
        ("projection_query", vec!["actor_skill"]),
        ("presentation_projector", vec!["choice_skill_prefix"]),
        ("markup", vec!["slow"]),
    ] {
        for name in names {
            assert!(
                rows.iter()
                    .any(|row| row.0 == name && row.1 == kind_label(kind)),
                "missing {kind}:{name}"
            );
        }
    }
    assert_eq!(kind_label("unsupported"), "");
    Ok(())
}

#[test]
fn parameter_signatures_use_author_facing_types_and_nested_list_names() {
    let params = [
        ("text", SchemaTypeRef::String),
        ("symbol", SchemaTypeRef::Symbol),
        ("integer", SchemaTypeRef::Int),
        ("number", SchemaTypeRef::Float),
        ("flag", SchemaTypeRef::Bool),
        ("actor", SchemaTypeRef::Speaker),
        ("mood", SchemaTypeRef::Enum("mood".into())),
        (
            "items",
            SchemaTypeRef::Array(Box::new(SchemaTypeRef::Registry("item".into()))),
        ),
    ]
    .map(|(name, type_ref)| ParameterDefinition {
        name: name.into(),
        type_ref,
    });
    assert_eq!(
        signature(&params),
        "text: Text\nsymbol: Symbol\ninteger: Integer\nnumber: Number\nflag: True or false\nactor: Speaker\nmood: mood\nitems: List of item"
    );
    assert_eq!(signature(&[]), "");
    assert_eq!(kind_label("condition"), "Condition");
    assert_eq!(kind_label("effect"), "Effect request");
    assert_eq!(kind_label("speaker"), "Speaker");
    assert_eq!(kind_label("registry"), "Registry");
}
