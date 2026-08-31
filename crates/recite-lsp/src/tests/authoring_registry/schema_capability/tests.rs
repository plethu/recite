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

#[test]
pub(super) fn schema_projection_uses_declaration_context_and_localized_selectors() {
    let catalog = localized_schema_catalog();
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
    schema.registries.insert(
        "items".to_owned(),
        recite_core::RegistryDefinition {
            origin: Some(recite_core::ProducerOrigin {
                kind: "adapter".to_owned(),
                id: "generated".to_owned(),
                label: None,
                extensions: std::collections::BTreeMap::new(),
            }),
            ..recite_core::RegistryDefinition::default()
        },
    );
    let evidence = recite_compiler::SchemaSummaryEvidence::builder(producer.clone())
        .capability(recite_compiler::ProducerCapabilityStatus::Supported)
        .build()
        .expect("evidence");
    let summary =
        recite_compiler::SchemaSummary::from_schema_with_evidence(&schema, Some(&evidence))
            .expect("summary");
    let actions =
        crate::features::code_action::schema_capability::actions(&summary, false, &catalog);
    let titles = actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .map(|action| action.title.clone())
        .collect::<Vec<_>>();
    assert!(
        titles
            .iter()
            .any(|title| title.contains("INVOKE") && title.contains("registry:items")),
        "{titles:?}"
    );
    let invoke = actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .find(|action| action.title.contains("INVOKE") && action.title.contains("registry:items"))
        .expect("registry declaration action");
    assert!(invoke.title.contains("adapter/generated"));

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
    let summary =
        recite_compiler::SchemaSummary::from_schema_with_evidence(&expected, Some(&evidence))
            .expect("summary");
    let detail = crate::features::schema_hover::hover_detail(None, &summary, &[], &catalog);
    assert!(detail.contains("STALE CONTENT-STALE"));
}

fn localized_schema_catalog() -> recite_ui::UiCatalog {
    let action = concat!(
        "lsp-code-action-schema-action = Schema capability ({$declaration}): { $action ->\n",
        "    [open-source] open source declaration\n",
        "    [edit-standalone] edit standalone source\n",
        "    [invoke] invoke producer\n",
        "    [retry] retry producer failure\n",
        "    [read-only] read-only generated schema\n",
        "    [unavailable] unavailable schema action\n",
        "   *[other] schema action\n",
        "} ({$producer})"
    );
    let freshness = concat!(
        "lsp-hover-schema-freshness-state =  Freshness { $state ->\n",
        "    [fresh] fresh\n",
        "    [stale] stale\n",
        "   *[other] unavailable\n",
        "}: content { $content ->\n",
        "    [fresh] fresh\n",
        "    [stale] stale\n",
        "   *[other] unavailable\n",
        "}; manifest { $manifest ->\n",
        "    [fresh] fresh\n",
        "    [stale] stale\n",
        "   *[other] unavailable\n",
        "}; registries {$registries}; metadata domains {$metadata_domains}."
    );
    let localized = recite_ui::DEFAULT_RESOURCE
        .replace(
            action,
            concat!(
                "lsp-code-action-schema-action = { $action ->\n",
                "    [invoke] INVOKE\n",
                "    [open-source] OPEN\n",
                "   *[other] OTHER\n",
                "} {$declaration} {$producer}"
            ),
        )
        .replace(
            freshness,
            concat!(
                "lsp-hover-schema-freshness-state = { $state ->\n",
                "    [stale] STALE\n",
                "   *[other] FRESH\n",
                "} { $content ->\n",
                "    [stale] CONTENT-STALE\n",
                "   *[other] CONTENT-FRESH\n",
                "}; manifest {$manifest}; registries {$registries}; metadata domains {$metadata_domains}."
            ),
        );
    recite_ui::UiCatalog::from_resources(
        "fr".parse().expect("locale"),
        [("en-US".parse().expect("locale"), localized)],
    )
    .expect("localized catalog")
}
