use super::*;

#[test]
fn line_lowering_preserves_ordered_metadata_and_speaker() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001 speaker=innkeeper portrait=neutral sfx=door sfx=mug repeat=true count=2\n",
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
        28
    );
    assert_eq!(
        first_metadata
            .source_span
            .as_ref()
            .unwrap()
            .end
            .unwrap()
            .column(),
        43
    );
    assert_eq!(first_metadata.key_span.as_ref().unwrap().start.column(), 28);
    assert_eq!(
        first_metadata.value_span.as_ref().unwrap().start.column(),
        37
    );
}

#[test]
fn malformed_block_header_fields_are_reported() {
    let source = concat!(
        ":: id=bad\n",
        ":: tavern_arrival default bare speaker=\n",
        "> ta_001\n",
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
        "> ta_001 portrait=\"neutral face\" tags=[door, \"mug clang\", true, 2, 1.5] sfx=door\n",
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
        19
    );
    assert_eq!(
        line.metadata.as_slice()[1]
            .value_span
            .as_ref()
            .unwrap()
            .start
            .column(),
        39
    );
}

#[test]
fn malformed_quoted_and_array_metadata_values_are_reported() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> bad_quote mood=\"unterminated\n",
        "  Hello.\n",
        "> bad_array tags=[door,]\n",
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
        "> bad_dollar mood=$hero\n",
        "  Hello.\n",
        "> bad_punctuation mood=hero!\n",
        "  Hello.\n",
        "> bad_comma mood=hero,alt\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE008", "RECITE_PARSE008", "RECITE_PARSE008"],
    );
    let block = single_block(&lowered);
    assert_eq!(line_statement(block, 0).metadata.len(), 0);
    assert_eq!(line_statement(block, 1).metadata.len(), 0);
    assert_eq!(line_statement(block, 2).metadata.len(), 0);
}
