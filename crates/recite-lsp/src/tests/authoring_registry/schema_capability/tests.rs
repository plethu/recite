#[test]
pub(super) fn schema_hover_exposes_compared_channels_and_unavailable_reasons() {
    let catalog =
        recite_ui::UiCatalog::load(&recite_ui::UiLocale::default()).expect("default UI catalog");
    let producer =
        recite_core::ProducerIdentity::new("adapter", "generated").expect("producer identity");
    let content_a = recite_core::producer_content_fingerprint(
        "blake3",
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .expect("content fingerprint");
    let content_b = recite_core::producer_content_fingerprint(
        "blake3",
        "1111111111111111111111111111111111111111111111111111111111111111",
    )
    .expect("content fingerprint");
    let metadata = |content| recite_core::ProducerMetadata {
        producer: Some(producer.clone()),
        content_fingerprint: Some(content),
        schema_export_version: None,
        inclusion_policy: None,
        producer_fingerprints: Vec::new(),
    };
    let mut expected = recite_core::ProjectSchema::empty_v1();
    expected.producer_metadata = Some(metadata(content_a));
    let mut actual = expected.clone();
    actual.producer_metadata = Some(metadata(content_b));
    let evidence = recite_compiler::SchemaSummaryEvidence::builder(producer)
        .compare_freshness(&expected, &actual)
        .expect("freshness comparison")
        .build()
        .expect("evidence");
    let compared =
        recite_compiler::SchemaSummary::from_schema_with_evidence(&expected, Some(&evidence))
            .expect("compared summary");
    let compared_detail =
        crate::features::schema_hover::hover_detail(None, &compared, &[], &catalog);
    assert!(compared_detail.contains("Freshness stale"));
    assert!(compared_detail.contains("content stale"));
    assert!(compared_detail.contains("manifest fresh"));
    assert!(compared_detail.contains("registries none"));
    assert!(compared_detail.contains("metadata domains none"));

    let no_producer =
        recite_compiler::SchemaSummary::from_schema(&recite_core::ProjectSchema::empty_v1());
    assert!(
        crate::features::schema_hover::hover_detail(None, &no_producer, &[], &catalog)
            .contains("no producer metadata")
    );
    assert!(
        crate::features::schema_hover::hover_detail(
            None,
            &recite_compiler::SchemaSummary::from_schema(&expected),
            &[],
            &catalog,
        )
        .contains("no comparison snapshot")
    );
}

#[test]
pub(super) fn schema_capability_projection_keeps_producer_actions_visible_and_disabled() {
    let catalog =
        recite_ui::UiCatalog::load(&recite_ui::UiLocale::default()).expect("default UI catalog");
    let producer =
        recite_core::ProducerIdentity::new("adapter", "generated").expect("producer identity");
    let mut schema = recite_core::ProjectSchema::empty_v1();
    schema.producer_metadata = Some(recite_core::ProducerMetadata {
        producer: Some(producer.clone()),
        content_fingerprint: None,
        schema_export_version: None,
        inclusion_policy: None,
        producer_fingerprints: Vec::new(),
    });
    let evidence = recite_compiler::SchemaSummaryEvidence::builder(producer.clone())
        .capability(recite_compiler::ProducerCapabilityStatus::Supported)
        .current_failure(
            recite_compiler::ProducerFailureEvidence::new(producer, "producer-exit", None)
                .expect("failure evidence"),
        )
        .build()
        .expect("supported failure evidence");
    let summary =
        recite_compiler::SchemaSummary::from_schema_with_evidence(&schema, Some(&evidence))
            .expect("summary");
    let actions =
        crate::features::code_action::schema_capability::actions(&summary, false, &catalog);
    let actions = actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(
        actions
            .iter()
            .any(|action| action.title.contains("invoke producer"))
    );
    assert!(
        actions
            .iter()
            .any(|action| action.title.contains("retry producer failure"))
    );
    assert!(actions.iter().all(|action| action.edit.is_none()));
    assert!(actions.iter().all(|action| action.disabled.is_some()));
}
