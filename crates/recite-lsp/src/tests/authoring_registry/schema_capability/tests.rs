mod freshness;

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
        !actions
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
                kind: "source".to_owned(),
                id: "registry".to_owned(),
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
            .any(|title| title.contains("INVOKE") && title.contains("REGISTRY items")),
        "{titles:?}"
    );
    let invoke = actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .find(|action| action.title.contains("INVOKE") && action.title.contains("REGISTRY items"))
        .expect("registry declaration action");
    assert!(invoke.title.contains("adapter/generated"));
    let open = actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .find(|action| action.title.contains("OPEN") && action.title.contains("REGISTRY items"))
        .expect("registry source action");
    assert!(open.title.contains("source/registry"));

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
    freshness::assert_absent_content_fingerprint(&catalog);

    let empty =
        recite_compiler::SchemaSummary::from_schema(&recite_core::ProjectSchema::empty_v1());
    let empty_actions =
        crate::features::code_action::schema_capability::actions(&empty, false, &catalog);
    let unavailable = empty_actions
        .iter()
        .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(action) => Some(action),
            lsp_types::CodeActionOrCommand::Command(_) => None,
        })
        .find(|action| action.title.contains("NO-PRODUCER"))
        .expect("unavailable action selector");
    assert!(
        unavailable
            .disabled
            .as_ref()
            .is_some_and(|reason| reason.reason.contains("OTHER"))
    );
    assert!(
        crate::features::schema_hover::hover_detail(None, &empty, &[], &catalog)
            .contains("NO-PRODUCER")
    );
}

pub(super) fn localized_schema_catalog() -> recite_ui::UiCatalog {
    let action = concat!(
        "lsp-code-action-schema-action = Schema capability ({ $declaration_kind ->\n",
        "    [type] type\n",
        "    [registry] registry\n",
        "    [speaker] speaker\n",
        "    [condition] condition\n",
        "    [reason] reason\n",
        "    [effect] effect\n",
        "    [metadata-domain] metadata domain\n",
        "    [metadata] metadata\n",
        "    [projection-query] projection query\n",
        "    [projector] projector\n",
        "    [markup] markup\n",
        "   *[schema] schema\n",
        "} {$declaration_name}): { $action ->\n",
        "    [open-source] open source declaration\n",
        "    [edit-standalone] edit standalone source\n",
        "    [invoke] invoke producer\n",
        "    [retry] retry producer failure\n",
        "    [read-only] read-only generated schema\n",
        "    [unavailable] unavailable schema action\n",
        "   *[other] schema action\n",
        "} ({ $producer_state ->\n",
        "    [present] {$producer}\n",
        "   *[absent] no producer {$producer}\n",
        "})"
    );
    let unavailable = concat!(
        "lsp-hover-schema-freshness-unavailable =  Freshness unavailable: { $reason ->\n",
        "    [no-comparison-snapshot] no comparison snapshot\n",
        "    [no-producer-metadata] no producer metadata\n",
        "   *[other] unavailable for this client\n",
        "}."
    );
    let disabled = concat!(
        "lsp-code-action-schema-disabled = Schema capability unavailable: { $reason ->\n",
        "    [source-location] source location is not available\n",
        "    [standalone-source-closed] standalone source is not open with a version\n",
        "    [standalone-edit] standalone source edit is not available\n",
        "    [producer-contract] producer execution contract is not available\n",
        "    [generated-read-only] generated schema is read-only\n",
        "    [unknown-source-owner] source owner is unknown\n",
        "    [producer-capability] producer capability is unavailable\n",
        "   *[other] schema action is not supported by this client\n",
        "}."
    );
    let freshness_evidence = concat!(
        "lsp-hover-schema-freshness =  Content fingerprint { $fingerprint_state ->\n",
        "    [present] {$fingerprint}\n",
        "   *[absent] none {$fingerprint}\n",
        "}; {$inputs} producer input fingerprints{$scope}."
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
                "lsp-code-action-schema-action = { $declaration_kind ->\n",
                "    [registry] REGISTRY\n",
                "   *[schema] SCHEMA\n",
                "} {$declaration_name} { $action ->\n",
                "    [invoke] INVOKE\n",
                "    [open-source] OPEN\n",
                "   *[other] OTHER\n",
                "} { $producer_state ->\n",
                "    [present] PRODUCER {$producer}\n",
                "   *[absent] NO-PRODUCER {$producer}\n",
                "}"
            ),
        )
        .replace(
            disabled,
            "lsp-code-action-schema-disabled = DISABLED { $reason ->\n    [generated-read-only] READONLY\n   *[other] OTHER\n}."
        )
        .replace(
            unavailable,
            "lsp-hover-schema-freshness-unavailable = { $reason ->\n    [no-producer-metadata] NO-PRODUCER\n   *[other] OTHER\n}."
        )
        .replace(
            freshness_evidence,
            "lsp-hover-schema-freshness = { $fingerprint_state ->\n    [present] CONTENT-PRESENT {$fingerprint}\n   *[absent] ABSENT-CONTENT {$fingerprint}\n}; {$inputs} {$scope}."
        )
        .replace(
            freshness,
            "lsp-hover-schema-freshness-state = { $state ->\n    [stale] STALE\n   *[other] FRESH\n} { $content ->\n    [stale] CONTENT-STALE\n   *[other] CONTENT-FRESH\n} {$manifest} {$registries} {$metadata_domains}."
        );
    recite_ui::UiCatalog::from_resources(
        "fr".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                recite_ui::DEFAULT_RESOURCE.to_owned(),
            ),
            ("fr".parse().expect("locale"), localized),
        ],
    )
    .expect("localized catalog")
}
