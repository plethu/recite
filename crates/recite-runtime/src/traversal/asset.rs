use std::ops::Range;

use recite_core::{
    BlockIndex, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex,
    ChoiceRange, CompiledAssetHeader, CompiledChoice, CompiledDialogue, CompiledLine,
    CompiledMetadataEntry, CompiledSourceMapEntry, CompiledStatement, CompiledValueError,
    LineIndex, MetadataIndex, MetadataRange, SourceMapIndex, SpeakerIndex, StatementIndex,
    StatementRange, TableRange,
};

use crate::{DialogueError, DialogueSession};

#[derive(Clone, Copy)]
pub(super) struct AssetView<'a> {
    asset: &'a CompiledDialogue,
}

impl<'a> AssetView<'a> {
    pub(super) fn new(asset: &'a CompiledDialogue) -> Result<Self, DialogueError> {
        ensure_supported_header(&asset.header)?;

        Ok(Self { asset })
    }

    pub(super) fn default_block(self) -> BlockIndex {
        self.asset.default_block
    }

    pub(super) fn ensure_session_matches(
        self,
        session: &DialogueSession,
    ) -> Result<(), DialogueError> {
        if session.asset_id != self.asset.header.asset_id
            || session.format_version != self.asset.header.format_version
            || session.compiler_compatibility_version
                != self.asset.header.compiler_compatibility_version
        {
            return Err(DialogueError::AssetMismatch {
                expected_asset_id: session.asset_id.as_str().to_owned(),
                actual_asset_id: self.asset.header.asset_id.as_str().to_owned(),
                expected_format_version: session.format_version,
                actual_format_version: self.asset.header.format_version,
                expected_compiler_compatibility_version: session.compiler_compatibility_version,
                actual_compiler_compatibility_version: self
                    .asset
                    .header
                    .compiler_compatibility_version,
            });
        }

        Ok(())
    }

    pub(super) fn lookup_block(self, block: &str) -> Result<BlockIndex, DialogueError> {
        let block_index = self
            .asset
            .block_lookup
            .as_slice()
            .binary_search_by(|entry| entry.id.as_str().cmp(block))
            .map(|index| self.asset.block_lookup.as_slice()[index].index)
            .map_err(|_| DialogueError::UnknownBlock {
                block: block.to_owned(),
            })?;
        let resolved_block = self.block_at(block_index)?;

        if resolved_block.id.as_str() != block {
            return Err(malformed(format!(
                "block lookup entry `{block}` points to block `{}` at index {}",
                resolved_block.id,
                block_index.as_u32()
            )));
        }

        Ok(block_index)
    }

    pub(super) fn block_at(
        self,
        index: BlockIndex,
    ) -> Result<&'a recite_core::CompiledBlock, DialogueError> {
        self.asset
            .blocks
            .get(index.as_u32() as usize)
            .ok_or_else(|| malformed(format!("block index {} is out of range", index.as_u32())))
    }

    pub(super) fn statement_at(
        self,
        index: StatementIndex,
    ) -> Result<&'a CompiledStatement, DialogueError> {
        self.asset
            .statements
            .get(index.as_u32() as usize)
            .ok_or_else(|| {
                malformed(format!(
                    "statement index {} is out of range",
                    index.as_u32()
                ))
            })
    }

    pub(super) fn line_at(self, index: LineIndex) -> Result<&'a CompiledLine, DialogueError> {
        self.asset
            .lines
            .get(index.as_u32() as usize)
            .ok_or_else(|| malformed(format!("line index {} is out of range", index.as_u32())))
    }

    pub(super) fn speaker_at(
        self,
        index: SpeakerIndex,
    ) -> Result<&'a recite_core::CompiledSpeaker, DialogueError> {
        self.asset
            .speakers
            .get(index.as_u32() as usize)
            .ok_or_else(|| malformed(format!("speaker index {} is out of range", index.as_u32())))
    }

    pub(super) fn source_map_at(
        self,
        index: SourceMapIndex,
    ) -> Result<&'a CompiledSourceMapEntry, DialogueError> {
        self.asset
            .source_maps
            .get(index.as_u32() as usize)
            .ok_or_else(|| {
                malformed(format!(
                    "source map index {} is out of range",
                    index.as_u32()
                ))
            })
    }

    pub(super) fn choices(self, range: ChoiceRange) -> Result<&'a [CompiledChoice], DialogueError> {
        let bounds = table_range(
            "choices",
            self.asset.choices.len(),
            range,
            ChoiceIndex::as_u32,
        )?;

        Ok(&self.asset.choices[bounds])
    }

    pub(super) fn metadata_entries(
        self,
        range: MetadataRange,
    ) -> Result<&'a [CompiledMetadataEntry], DialogueError> {
        let bounds = table_range(
            "metadata",
            self.asset.metadata.len(),
            range,
            MetadataIndex::as_u32,
        )?;

        Ok(&self.asset.metadata[bounds])
    }

    pub(super) fn statement_range(
        self,
        range: StatementRange,
    ) -> Result<Range<usize>, DialogueError> {
        table_range(
            "statements",
            self.asset.statements.len(),
            range,
            StatementIndex::as_u32,
        )
    }
}

fn ensure_supported_header(header: &CompiledAssetHeader) -> Result<(), DialogueError> {
    if header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0
        || header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0
    {
        return Err(DialogueError::UnsupportedCompiledFormat {
            format_version: header.format_version,
            compiler_compatibility_version: header.compiler_compatibility_version,
        });
    }

    Ok(())
}

fn table_range<I: Copy>(
    table: &'static str,
    table_len: usize,
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> Result<Range<usize>, DialogueError> {
    let start = index(range.start) as usize;
    let len = range.len as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| malformed(format!("{table} range overflows usize")))?;

    if end > table_len {
        return Err(malformed(format!(
            "{table} range {start}..{end} exceeds table length {table_len}"
        )));
    }

    Ok(start..end)
}

pub(super) fn malformed(reason: String) -> DialogueError {
    DialogueError::MalformedCompiledAsset { reason }
}

impl From<CompiledValueError> for DialogueError {
    fn from(error: CompiledValueError) -> Self {
        malformed(error.to_string())
    }
}
