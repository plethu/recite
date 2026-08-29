use super::*;

#[test]
fn line_lowering_preserves_ordered_metadata_and_speaker() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001@a7091198ef21b28b9e4b speaker=innkeeper portrait=neutral sfx=door sfx=mug repeat=true count=2\n",
        "  Welcome.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let line = line_statement(single_block(&lowered), 0);

    assert_eq!(
        line.speaker.as_ref().map(SpeakerId::as_str),
        Some("innkeeper")
    );
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.value))
            .collect::<Vec<_>>(),
        [
            (
                "portrait",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol("neutral".to_owned()))
            ),
            (
                "sfx",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol("door".to_owned()))
            ),
            (
                "sfx",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol("mug".to_owned()))
            ),
            (
                "repeat",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Bool(true))
            ),
            (
                "count",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Integer(2))
            ),
        ]
    );

    let first_metadata = &line.metadata.as_slice()[0];
    assert_eq!(
        first_metadata.source_span.as_ref().unwrap().start.column(),
        49
    );
    assert_eq!(
        first_metadata
            .source_span
            .as_ref()
            .unwrap()
            .end
            .unwrap()
            .column(),
        64
    );
    assert_eq!(first_metadata.key_span.as_ref().unwrap().start.column(), 49);
    assert_eq!(
        first_metadata.value_span.as_ref().unwrap().start.column(),
        58
    );
}

#[test]
fn malformed_block_header_fields_are_reported() {
    let source = concat!(
        ":: id=bad\n",
        ":: tavern_arrival default bare speaker=\n",
        "> ta_001@2dfe95f5bf35d2701638\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE003", "RECITE_PARSE008", "RECITE_PARSE008"],
    );
    let block = single_block(&lowered);
    assert_eq!(block.id.as_str(), "tavern_arrival");
    assert!(block.is_default);
    assert!(block.default_speaker.is_none());
}

#[test]
fn metadata_values_support_quotes_with_spaces_and_arrays() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001@c30dd0f5fc77ca33a5c2 portrait=\"neutral face\" tags=[door, \"mug clang\", true, 2, 1.5] sfx=door\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.value))
            .collect::<Vec<_>>(),
        [
            (
                "portrait",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::StringLiteral(
                    "neutral face".to_owned()
                ))
            ),
            (
                "tags",
                &SourceMetadataValue::Array(vec![
                    SourceMetadataScalar::Symbol("door".to_owned()),
                    SourceMetadataScalar::StringLiteral("mug clang".to_owned()),
                    SourceMetadataScalar::Bool(true),
                    SourceMetadataScalar::Integer(2),
                    SourceMetadataScalar::Float(1.5),
                ])
            ),
            (
                "sfx",
                &SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol("door".to_owned()))
            ),
        ]
    );
    assert_eq!(
        line.metadata.as_slice()[0]
            .value_span
            .as_ref()
            .unwrap()
            .start
            .column(),
        40
    );
    assert_eq!(
        line.metadata.as_slice()[1]
            .value_span
            .as_ref()
            .unwrap()
            .start
            .column(),
        60
    );
}

#[test]
fn malformed_quoted_and_array_metadata_values_are_reported() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> bad_quote@79a4460f7cefb74ac6ee mood=\"unterminated\n",
        "  Hello.\n",
        "> bad_array@26771660fd063c3a8d12 tags=[door,]\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE008", "RECITE_PARSE008"]);
    let block = single_block(&lowered);
    assert_eq!(line_statement(block, 0).metadata.len(), 0);
    assert_eq!(line_statement(block, 1).metadata.len(), 0);
}

#[test]
fn malformed_bare_symbol_metadata_values_are_reported() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> bad_dollar@1c52c1e138cce69c2665 mood=$hero\n",
        "  Hello.\n",
        "> bad_punctuation@cd04b8c2c6bc85aa9020 mood=hero!\n",
        "  Hello.\n",
        "> bad_comma@03ed2d9d217af362fbea mood=hero,alt\n",
        "  Hello.\n",
        "> bad_paren@14ed2d9d217af362fbea mood=hero)\n",
        "  Hello.\n",
        "> bad_bracket@25ed2d9d217af362fbea mood=hero]\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
        ],
    );
    let block = single_block(&lowered);
    assert_eq!(line_statement(block, 0).metadata.len(), 0);
    assert_eq!(line_statement(block, 1).metadata.len(), 0);
    assert_eq!(line_statement(block, 2).metadata.len(), 0);
    assert_eq!(line_statement(block, 3).metadata.len(), 0);
    assert_eq!(line_statement(block, 4).metadata.len(), 0);
}
