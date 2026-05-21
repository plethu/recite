use recite_core::{SchemaTypeDefinition, load_schema_manifest_str};

use crate::diagnostic_codes;

#[test]
fn map_like_manifest_sections_are_canonicalized_deterministically() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/unsorted_manifest.json",
        include_str!("../../../../fixtures/schema/valid/unsorted_manifest.json"),
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid schema manifest");

    assert_eq!(
        schema.types.keys().map(String::as_str).collect::<Vec<_>>(),
        ["a_state", "z_state"]
    );
    let SchemaTypeDefinition::Enum(z_state) = &schema.types["z_state"];
    assert_eq!(
        z_state
            .values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "m", "z"]
    );
    assert_eq!(
        schema
            .registries
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a_registry", "z_registry"]
    );
    assert_eq!(
        schema.registries["z_registry"]
            .values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one", "three", "two"]
    );
}
