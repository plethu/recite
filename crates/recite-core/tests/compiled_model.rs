use recite_core::{
    BLAKE3_DIGEST_LEN, BlockId, BlockIndex, BlockLookupEntry, BlockLookupTable,
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, ChoiceId, ChoiceIndex,
    ChoiceLookupEntry, ChoiceLookupTable, ChoiceRange, CompiledAssetEncoding, CompiledAssetHeader,
    CompiledAssetId, CompiledChoice, CompiledChoiceEcho, CompiledDialogue, CompiledDivertTarget,
    CompiledInspectionEncoding, CompiledLine, CompiledMatchArm, CompiledMatchPattern,
    CompiledMetadataEntry, CompiledSourceMapEntry, CompiledStatement, CompiledStatementKind,
    CompiledValueError, CompilerVersion, ContentFingerprint, LineId, LineIndex, LineLookupEntry,
    LineLookupTable, MatchArmIndex, MatchArmRange, MetadataIndex, MetadataRange, ScalarValue,
    SchemaFingerprint, SourceFileIndex, SourceMapId, SourceMapIndex, SourcePosition, SourceSpan,
    SpeakerIndex, StatementIndex, StatementRange, V0_ARGUMENT_TAG_IDENTIFIER,
    V0_ARGUMENT_TAG_VALUE, V0_ASSET_ENCODING_MESSAGEPACK, V0_ASSET_HEADER_FIELDS,
    V0_CHOICE_ECHO_TAG_EXPLICIT_LINE, V0_CHOICE_ECHO_TAG_NONE, V0_CHOICE_ECHO_TAG_SELECTED_TEXT,
    V0_COMPILED_DIALOGUE_FIELDS, V0_CONDITION_TAG_AND, V0_CONDITION_TAG_CALL, V0_CONDITION_TAG_NOT,
    V0_CONDITION_TAG_OR, V0_DIVERT_TARGET_TAG_BLOCK, V0_DIVERT_TARGET_TAG_END,
    V0_EFFECT_MODE_TAG_BLOCKING, V0_EFFECT_MODE_TAG_DEFERRED, V0_EFFECT_MODE_TAG_IMMEDIATE,
    V0_INSPECTION_ENCODING_COMPACT_JSON, V0_LOOKUP_ENTRY_FIELDS, V0_MATCH_ARM_FIELDS,
    V0_MATCH_PATTERN_TAG_VARIANT, V0_MATCH_PATTERN_TAG_WILDCARD, V0_RANGE_FIELDS,
    V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
    V0_SOURCE_SPAN_FIELDS, V0_STATEMENT_TAG_DIVERT, V0_STATEMENT_TAG_EFFECT, V0_STATEMENT_TAG_END,
    V0_STATEMENT_TAG_IF, V0_STATEMENT_TAG_LINE, V0_STATEMENT_TAG_MATCH, V0_STATEMENT_TAG_PROMPT,
    Value,
};

#[test]
fn v0_wire_constants_lock_main_tuple_and_tag_decisions() {
    assert_eq!(V0_COMPILED_DIALOGUE_FIELDS, 14);
    assert_eq!(V0_ASSET_HEADER_FIELDS, 8);
    assert_eq!(V0_MATCH_ARM_FIELDS, 3);
    assert_eq!(V0_RANGE_FIELDS, 2);
    assert_eq!(V0_LOOKUP_ENTRY_FIELDS, 2);
    assert_eq!(V0_SOURCE_SPAN_FIELDS, 5);

    assert_eq!(V0_ASSET_ENCODING_MESSAGEPACK, 0);
    assert_eq!(V0_INSPECTION_ENCODING_COMPACT_JSON, 0);
    assert_eq!(V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, 0);
    assert_eq!(V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA, 1);

    assert_eq!(V0_STATEMENT_TAG_LINE, 0);
    assert_eq!(V0_STATEMENT_TAG_PROMPT, 1);
    assert_eq!(V0_STATEMENT_TAG_DIVERT, 2);
    assert_eq!(V0_STATEMENT_TAG_IF, 3);
    assert_eq!(V0_STATEMENT_TAG_MATCH, 4);
    assert_eq!(V0_STATEMENT_TAG_EFFECT, 5);
    assert_eq!(V0_STATEMENT_TAG_END, 6);
    assert_eq!(V0_MATCH_PATTERN_TAG_VARIANT, 0);
    assert_eq!(V0_MATCH_PATTERN_TAG_WILDCARD, 1);

    assert_eq!(V0_DIVERT_TARGET_TAG_BLOCK, 0);
    assert_eq!(V0_DIVERT_TARGET_TAG_END, 1);
    assert_eq!(V0_CHOICE_ECHO_TAG_NONE, 0);
    assert_eq!(V0_CHOICE_ECHO_TAG_SELECTED_TEXT, 1);
    assert_eq!(V0_CHOICE_ECHO_TAG_EXPLICIT_LINE, 2);

    assert_eq!(V0_EFFECT_MODE_TAG_DEFERRED, 0);
    assert_eq!(V0_EFFECT_MODE_TAG_IMMEDIATE, 1);
    assert_eq!(V0_EFFECT_MODE_TAG_BLOCKING, 2);
    assert_eq!(V0_CONDITION_TAG_CALL, 0);
    assert_eq!(V0_CONDITION_TAG_AND, 1);
    assert_eq!(V0_CONDITION_TAG_OR, 2);
    assert_eq!(V0_CONDITION_TAG_NOT, 3);
    assert_eq!(V0_ARGUMENT_TAG_IDENTIFIER, 0);
    assert_eq!(V0_ARGUMENT_TAG_VALUE, 1);
}

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
    assert_eq!(fingerprint.digest().as_bytes(), &[1; BLAKE3_DIGEST_LEN]);
}

#[test]
fn blake3_fingerprints_accept_exactly_32_byte_digests() {
    let fingerprint =
        ContentFingerprint::blake3([1; BLAKE3_DIGEST_LEN]).expect("valid blake3 digest");

    assert_eq!(fingerprint.algorithm().as_str(), "blake3");
    assert_eq!(fingerprint.digest().as_bytes().len(), BLAKE3_DIGEST_LEN);
}

#[test]
fn constrained_compiled_values_reject_empty_strings_and_invalid_digests() {
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
    assert_eq!(
        ContentFingerprint::blake3([1; BLAKE3_DIGEST_LEN - 1]),
        Err(CompiledValueError::InvalidFingerprintDigestLength {
            algorithm: "blake3",
            expected: BLAKE3_DIGEST_LEN,
            actual: BLAKE3_DIGEST_LEN - 1,
        })
    );
    assert_eq!(
        ContentFingerprint::blake3([1; BLAKE3_DIGEST_LEN + 1]),
        Err(CompiledValueError::InvalidFingerprintDigestLength {
            algorithm: "blake3",
            expected: BLAKE3_DIGEST_LEN,
            actual: BLAKE3_DIGEST_LEN + 1,
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
fn match_statements_use_explicit_scrutinee_and_arm_tables() {
    let arm = CompiledMatchArm {
        pattern: CompiledMatchPattern::Variant("tired".to_owned()),
        statements: StatementRange::new(StatementIndex::new(4), 2),
        source_map: SourceMapIndex::new(2),
    };
    let fallback = CompiledMatchArm {
        pattern: CompiledMatchPattern::Wildcard,
        statements: StatementRange::new(StatementIndex::new(6), 1),
        source_map: SourceMapIndex::new(3),
    };
    let statement = CompiledStatement {
        kind: CompiledStatementKind::Match {
            scrutinee: recite_core::CompiledConditionCall {
                function: "thread_stage".to_owned(),
                args: vec![recite_core::CompiledArgument::Identifier(
                    "rhea_job_response".to_owned(),
                )],
            },
            arms: MatchArmRange::new(MatchArmIndex::new(0), 2),
        },
        source_map: SourceMapIndex::new(1),
    };

    assert_eq!(
        arm.pattern,
        CompiledMatchPattern::Variant("tired".to_owned())
    );
    assert_eq!(fallback.pattern, CompiledMatchPattern::Wildcard);

    let CompiledStatementKind::Match { scrutinee, arms } = statement.kind else {
        panic!("expected match statement");
    };
    assert_eq!(scrutinee.function, "thread_stage");
    assert_eq!(arms.start, MatchArmIndex::new(0));
    assert_eq!(arms.len, 2);
}

#[test]
fn lookup_table_wrappers_accept_sorted_unique_rows() {
    let blocks = BlockLookupTable::new(vec![
        BlockLookupEntry {
            id: BlockId::new("intro").expect("valid block id"),
            index: BlockIndex::new(0),
        },
        BlockLookupEntry {
            id: BlockId::new("work").expect("valid block id"),
            index: BlockIndex::new(1),
        },
    ])
    .expect("sorted unique block lookup");
    let lines = LineLookupTable::new(vec![
        LineLookupEntry {
            id: LineId::new("intro_001").expect("valid line id"),
            index: LineIndex::new(0),
        },
        LineLookupEntry {
            id: LineId::new("work_001").expect("valid line id"),
            index: LineIndex::new(1),
        },
    ])
    .expect("sorted unique line lookup");
    let choices = ChoiceLookupTable::new(vec![
        ChoiceLookupEntry {
            id: ChoiceId::new("ask_work").expect("valid choice id"),
            index: ChoiceIndex::new(0),
        },
        ChoiceLookupEntry {
            id: ChoiceId::new("leave").expect("valid choice id"),
            index: ChoiceIndex::new(1),
        },
    ])
    .expect("sorted unique choice lookup");

    assert_eq!(blocks.len(), 2);
    assert_eq!(
        blocks
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["intro", "work"]
    );
    assert_eq!(lines.as_slice()[1].index, LineIndex::new(1));
    assert_eq!(choices.as_slice()[0].id.as_str(), "ask_work");
}

#[test]
fn lookup_table_wrappers_reject_duplicate_and_unsorted_rows() {
    assert_eq!(
        BlockLookupTable::new(vec![
            BlockLookupEntry {
                id: BlockId::new("work").expect("valid block id"),
                index: BlockIndex::new(1),
            },
            BlockLookupEntry {
                id: BlockId::new("intro").expect("valid block id"),
                index: BlockIndex::new(0),
            },
        ]),
        Err(CompiledValueError::UnsortedLookupTable {
            table: "block",
            previous: "work".to_owned(),
            current: "intro".to_owned(),
        })
    );
    assert_eq!(
        ChoiceLookupTable::new(vec![
            ChoiceLookupEntry {
                id: ChoiceId::new("ask_work").expect("valid choice id"),
                index: ChoiceIndex::new(0),
            },
            ChoiceLookupEntry {
                id: ChoiceId::new("ask_work").expect("valid choice id"),
                index: ChoiceIndex::new(1),
            },
        ]),
        Err(CompiledValueError::UnsortedLookupTable {
            table: "choice",
            previous: "ask_work".to_owned(),
            current: "ask_work".to_owned(),
        })
    );
}

#[test]
fn compiled_dialogue_uses_typed_lookup_tables() {
    let dialogue = CompiledDialogue {
        header: CompiledAssetHeader::messagepack_v0(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            SchemaFingerprint::NoSchema,
        ),
        sources: Vec::new(),
        blocks: Vec::new(),
        statements: Vec::new(),
        match_arms: Vec::new(),
        lines: Vec::new(),
        choices: Vec::new(),
        speakers: Vec::new(),
        metadata: Vec::new(),
        effects: Vec::new(),
        source_maps: Vec::new(),
        block_lookup: BlockLookupTable::default(),
        line_lookup: LineLookupTable::default(),
        choice_lookup: ChoiceLookupTable::default(),
    };

    assert!(dialogue.block_lookup.is_empty());
    assert!(dialogue.line_lookup.is_empty());
    assert!(dialogue.choice_lookup.is_empty());
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
