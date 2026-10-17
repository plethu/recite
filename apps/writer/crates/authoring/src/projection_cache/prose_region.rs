//! Reuse checked projection output outside a parser-owned restart region.
use super::ProjectionCache;
use crate::{Document, Passage, ScriptBlock};
use recite_parser::{SourceRegion, parse, source_regions};
use std::{cell::OnceCell, ops::Range, sync::Arc};

mod relocation;
use relocation::Relocation;

impl Document {
    /// Uncertain partition, identity or coordinate mappings use full validation.
    pub(crate) fn prose_region_projection(
        &self,
        next: &str,
        replacement: &Range<usize>,
        id: &str,
        text: &str,
    ) -> Option<ProjectionCache> {
        let previous = source_regions(self.source());
        if previous.len() < 2 {
            return None;
        }
        let current = source_regions(next);
        let index = matching_region(&previous, &current, replacement)?;
        let old = &previous[index];
        let new = &current[index];
        let passages = self.passage_snapshot().ok()?;
        let range = passage_range(&passages, old, previous.get(index + 1));
        // Local parsing keeps both CST bytes and lowered positions region-relative.
        let parsed = parse(self.key().as_str(), new.text());
        let lowered = parsed.lower_source_file();
        let mut changed = crate::projection::passages_from(new.text(), &parsed, &lowered).ok()?;
        if !changed
            .iter()
            .any(|passage| passage.id == id && passage.text == text)
            || !changed
                .iter()
                .map(|p| &p.id)
                .eq(passages[range.clone()].iter().map(|p| &p.id))
        {
            return None;
        }
        // The old whole-file projection established uniqueness. Identical ordered
        // IDs and byte-identical other regions preserve that proof.
        let local = Relocation::origin(new);
        let later = Relocation::after(old, new, previous.get(index + 1), current.get(index + 1))?;
        let script = self.projections.script.get().map(|script| {
            let summary = self.kernel().snapshot().document(self.key())?.summary();
            let blocks = summary.blocks();
            // Plural lines are absent from editable passages but Script still
            // resolves their IDs globally. Keep that case on the full path.
            if summary.stable_ids().len() != passages.len()
                || blocks.len() != script.len()
                || !blocks
                    .iter()
                    .zip(script.iter())
                    .all(|(block, script)| block.id().as_str() == script.id)
            {
                return None;
            }
            let start =
                blocks.partition_point(|block| block.span().start.line() < old.first_line());
            let end = previous.get(index + 1).map_or(blocks.len(), |next| {
                blocks.partition_point(|block| block.span().start.line() < next.first_line())
            });
            let mut changed = crate::script::blocks(new.text(), &lowered, &changed);
            if !changed
                .iter()
                .map(|block| (&block.id, block.is_default))
                .eq(script[start..end]
                    .iter()
                    .map(|block| (&block.id, block.is_default)))
            {
                return None;
            }
            for block in &mut changed {
                local.block(block)?;
            }
            joined_blocks(script, start..end, changed, &later)
        });
        let script = match script {
            Some(script) => Some(script?),
            None => None,
        };
        for passage in &mut changed {
            local.passage(passage)?;
        }
        let mut joined = Vec::with_capacity(passages.len());
        joined.extend_from_slice(&passages[..range.start]);
        joined.extend(changed);
        for passage in &passages[range.end..] {
            let mut passage = passage.clone();
            later.passage(&mut passage)?;
            joined.push(passage);
        }
        Some(ProjectionCache {
            passages: OnceCell::from(Arc::<[Passage]>::from(joined)),
            script: script.map_or_else(OnceCell::new, |script| {
                OnceCell::from(Arc::<[ScriptBlock]>::from(script))
            }),
            // Ordered editable IDs are unchanged, so indexes still name the
            // same slots in the newly owned passage array.
            ids: self.projections.ids.clone(),
        })
    }
}

fn matching_region(
    previous: &[SourceRegion<'_>],
    current: &[SourceRegion<'_>],
    replacement: &Range<usize>,
) -> Option<usize> {
    if previous.len() != current.len() {
        return None;
    }
    let index = previous.iter().position(|region| {
        let bytes = region.byte_range();
        bytes.start <= replacement.start && replacement.end <= bytes.end
    })?;
    let old = &previous[index];
    let new = &current[index];
    if old.first_line() != new.first_line() || old.byte_range().start != new.byte_range().start {
        return None;
    }
    previous
        .iter()
        .zip(current)
        .enumerate()
        .all(|(i, (old, new))| i == index || old.text() == new.text())
        .then_some(index)
}

fn passage_range(
    passages: &[Passage],
    region: &SourceRegion<'_>,
    next: Option<&SourceRegion<'_>>,
) -> Range<usize> {
    let start = passages.partition_point(|p| p.header_line < region.first_line());
    let end = next.map_or(passages.len(), |next| {
        passages.partition_point(|p| p.header_line < next.first_line())
    });
    start..end
}

fn joined_blocks(
    previous: &[ScriptBlock],
    range: Range<usize>,
    changed: Vec<ScriptBlock>,
    later: &Relocation,
) -> Option<Vec<ScriptBlock>> {
    let mut joined = Vec::with_capacity(previous.len());
    joined.extend_from_slice(&previous[..range.start]);
    joined.extend(changed);
    for block in &previous[range.end..] {
        let mut block = block.clone();
        later.block(&mut block)?;
        joined.push(block);
    }
    Some(joined)
}
