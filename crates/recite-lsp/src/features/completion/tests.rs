use super::*;
use recite_compiler::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument};
use recite_core::schema::load_schema_manifest_str;
use recite_ui::UiLocale;

#[test]
fn unsorted_schema_input_completes_every_condition_and_effect_with_details()
-> Result<(), Box<dyn std::error::Error>> {
    let report = load_schema_manifest_str(
        "schema.json",
        r#"{
        "schema_version": 1,
        "conditions": { "gate_z": {"params": []}, "gate_a": {"params": []} },
        "effects": { "action_z": {"params": [], "modes": ["immediate"]},
                     "action_a": {"params": [], "modes": ["immediate"]} }
    }"#,
    );
    let schema = report.schema.ok_or("schema")?;
    let summary = SchemaSummary::from_schema(&schema);
    let mut kernel = AuthoringKernel::with_schema(schema)?;
    let key = DocumentKey::new("source.recite")?;
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            key.clone(),
            ":: start\n:if gate_\n! immediate action_\n",
        )],
        [],
    ))?;
    let catalog = UiCatalog::load(&UiLocale::default())?;
    let control = recite_compiler::authoring::CancellationToken::new();
    let query = kernel.snapshot().query(&control);
    for (position, names) in [
        (Position::new(1, 9), ["gate_a", "gate_z"]),
        (Position::new(2, 19), ["action_a", "action_z"]),
    ] {
        let Some(CompletionResponse::Array(items)) =
            completion(position, Some(&key), &query, Some(&summary), &catalog)
        else {
            return Err("completion items".into());
        };
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            names
        );
        assert!(
            items
                .iter()
                .all(|i| i.detail.is_some() && i.documentation.is_some())
        );
    }
    Ok(())
}
