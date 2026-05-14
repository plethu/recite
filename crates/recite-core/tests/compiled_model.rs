use recite_core::{
    BlockId, BlockIndex, BlockLookupEntry, COMPILED_ASSET_FORMAT_VERSION_V0,
    COMPILER_COMPATIBILITY_VERSION_V0, ChoiceId, ChoiceIndex, ChoiceRange, CompiledAssetEncoding,
    CompiledAssetHeader, CompiledAssetId, CompiledChoice, CompiledChoiceEcho, CompiledDivertTarget,
    CompiledInspectionEncoding, CompiledLine, CompiledMetadataEntry, CompiledSourceMapEntry,
    CompiledStatement, CompiledStatementKind, CompiledValueError, CompilerVersion,
    ContentFingerprint, LineId, LineIndex, MetadataIndex, MetadataRange, ScalarValue,
    SchemaFingerprint, SourceFileIndex, SourceMapId, SourceMapIndex, SourcePosition, SourceSpan,
    SpeakerIndex, StatementIndex, StatementRange, Value,
};

#[test]
fn v0_header_locks_messagepack_and_freshness_fields() {
    let header = CompiledAssetHeader::messagepack_v0(
        CompilerVersion::new("0.0.1").expect("valid compiler version"),
        CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
        SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
        SchemaFingerprint::Fingerprint(
            ContentFingerprint::blake3([1; 32]).expect("valid fingerprint"),
        ),
    );

    assert_eq!(header.format_version, COMPILED_ASSET_FORMAT_VERSION_V0);
    assert_eq!(
        header.compiler_compatibility_version,
        COMPILER_COMPATIBILITY_VERSION_V0
    );
    assert_eq!(header.primary_encoding, CompiledAssetEncoding::MessagePack);
    assert_eq!(
        header.inspection_encoding,
        CompiledInspectionEncoding::CompactJson
    );

    let SchemaFingerprint::Fingerprint(fingerprint) = &header.schema_fingerprint else {
        panic!("expected schema fingerprint");
    };
    assert_eq!(fingerprint.algorithm().as_str(), "blake3");
    assert_eq!(fingerprint.digest().as_bytes(), &[1; 32]);
}

#[test]
fn constrained_compiled_values_reject_empty_strings_and_digests() {
    assert_eq!(
        CompilerVersion::new(" "),
        Err(CompiledValueError::EmptyValue {
            kind: "CompilerVersion"
        })
    );
    assert_eq!(
        ContentFingerprint::blake3([]),
        Err(CompiledValueError::EmptyValue {
            kind: "FingerprintDigest"
        })
    );
}

#[test]
fn compiled_rows_require_stable_line_and_choice_ids() {
    let line = CompiledLine {
        id: LineId::new("intro_001").expect("valid line id"),
        source_text: "Welcome.".to_owned(),
        speaker: Some(SpeakerIndex::new(0)),
        metadata: MetadataRange::new(MetadataIndex::new(0), 0),
        source_map: SourceMapIndex::new(0),
    };
    let choice = CompiledChoice {
        id: ChoiceId::new("ask_work").expect("valid choice id"),
        source_text: "Ask about work.".to_owned(),
        metadata: MetadataRange::new(MetadataIndex::new(0), 0),
        condition: None,
        target: CompiledDivertTarget::Block(BlockIndex::new(1)),
        echo: CompiledChoiceEcho::None,
        source_map: SourceMapIndex::new(1),
    };

    assert_eq!(line.id.as_str(), "intro_001");
    assert_eq!(choice.id.as_str(), "ask_work");
}

#[test]
fn metadata_rows_preserve_source_order_and_repeated_keys() {
    let metadata = [
        CompiledMetadataEntry {
            key: "mood".to_owned(),
            value: Value::from(ScalarValue::from("calm")),
            source_map: Some(SourceMapIndex::new(0)),
        },
        CompiledMetadataEntry {
            key: "portrait".to_owned(),
            value: Value::from(ScalarValue::from("neutral")),
            source_map: Some(SourceMapIndex::new(1)),
        },
        CompiledMetadataEntry {
            key: "mood".to_owned(),
            value: Value::from(ScalarValue::from("alert")),
            source_map: Some(SourceMapIndex::new(2)),
        },
    ];

    let keys = metadata
        .iter()
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(keys, ["mood", "portrait", "mood"]);
}

#[test]
fn ranges_and_lookup_rows_make_runtime_traversal_explicit() {
    let block = recite_core::CompiledBlock {
        id: BlockId::new("start").expect("valid block id"),
        source_file: SourceFileIndex::new(0),
        statements: StatementRange::new(StatementIndex::new(0), 2),
        metadata: MetadataRange::new(MetadataIndex::new(0), 0),
        default_speaker: None,
        source_map: SourceMapIndex::new(0),
    };
    let prompt = CompiledStatement {
        kind: CompiledStatementKind::Prompt {
            line: Some(LineIndex::new(0)),
            choices: ChoiceRange::new(ChoiceIndex::new(0), 2),
        },
        source_map: SourceMapIndex::new(1),
    };
    let lookup = BlockLookupEntry {
        id: BlockId::new("start").expect("valid block id"),
        index: BlockIndex::new(0),
    };

    assert_eq!(block.statements.start.as_u32(), 0);
    assert_eq!(block.statements.len, 2);
    assert!(!block.statements.is_empty());
    assert_eq!(lookup.index, BlockIndex::new(0));

    let CompiledStatementKind::Prompt { line, choices } = prompt.kind else {
        panic!("expected prompt");
    };
    assert_eq!(line, Some(LineIndex::new(0)));
    assert_eq!(choices.start, ChoiceIndex::new(0));
    assert_eq!(choices.len, 2);
}

#[test]
fn source_maps_keep_structured_source_spans() {
    let span = SourceSpan::new(
        "dialogue/start.recite",
        SourcePosition::new(3, 1).expect("valid start"),
        Some(SourcePosition::new(3, 12).expect("valid end")),
    );
    let source_map = CompiledSourceMapEntry {
        source_file: SourceFileIndex::new(0),
        span,
    };

    assert_eq!(source_map.source_file, SourceFileIndex::new(0));
    assert_eq!(source_map.span.file, "dialogue/start.recite");
    assert_eq!(source_map.span.start.line(), 3);
}
