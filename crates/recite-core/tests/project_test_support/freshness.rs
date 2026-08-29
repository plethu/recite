use recite_core::{
    BlockId, BlockIndex, BlockLookupEntry, BlockLookupTable, ChoiceLookupTable,
    CompiledAssetHeader, CompiledAssetId, CompiledDialogue, CompiledSourceFile, CompiledSpeaker,
    CompilerVersion, LineLookupTable, ProjectManifest, SchemaFingerprint, SourceMapId, SpeakerId,
    canonical_source_fingerprint,
};

pub(crate) fn manifest_source() -> recite_core::ProjectManifestSource {
    let report = ProjectManifest::load_str_with_spans(
        "recite.project.toml",
        "[[scenes]]\nid = \"opening\"\nasset = \"dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    report
        .source
        .unwrap_or_else(|| panic!("valid project source"))
}

pub(crate) fn asset_with(
    format_version: u16,
    compiler_compatibility_version: u16,
    schema_fingerprint: SchemaFingerprint,
    sources: Vec<CompiledSourceFile>,
    block_ids: &[&str],
    speaker_ids: &[&str],
) -> CompiledDialogue {
    let blocks = block_ids
        .iter()
        .enumerate()
        .map(|(index, id)| BlockLookupEntry {
            id: BlockId::new(*id).unwrap_or_else(|error| panic!("valid block id: {error}")),
            index: BlockIndex::new(index as u32),
        })
        .collect::<Vec<_>>();
    let header = CompiledAssetHeader::messagepack_v0(
        CompilerVersion::new("0.0.1")
            .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
        CompiledAssetId::new("dialogue.recitec")
            .unwrap_or_else(|error| panic!("valid asset id: {error}")),
        SourceMapId::new("dialogue.map")
            .unwrap_or_else(|error| panic!("valid source map id: {error}")),
        schema_fingerprint,
    );
    CompiledDialogue {
        header: CompiledAssetHeader {
            format_version,
            compiler_compatibility_version,
            ..header
        },
        default_block: BlockIndex::new(0),
        sources,
        blocks: Vec::new(),
        statements: Vec::new(),
        match_arms: Vec::new(),
        lines: Vec::new(),
        choices: Vec::new(),
        availability_reasons: Vec::new(),
        condition_availability_reasons: Vec::new(),
        speakers: speaker_ids
            .iter()
            .map(|id| CompiledSpeaker {
                id: SpeakerId::new(*id).unwrap_or_else(|error| panic!("valid speaker id: {error}")),
            })
            .collect(),
        metadata: Vec::new(),
        effects: Vec::new(),
        source_maps: Vec::new(),
        block_lookup: BlockLookupTable::new(blocks)
            .unwrap_or_else(|error| panic!("sorted block lookup: {error}")),
        line_lookup: LineLookupTable::default(),
        choice_lookup: ChoiceLookupTable::default(),
    }
}

pub(crate) fn source_fingerprint(value: &str) -> recite_core::ContentFingerprint {
    canonical_source_fingerprint(value)
}
