use std::collections::{BTreeMap, BTreeSet};

use recite_core::{Diagnostic, SourceFile, SourcePosition, SourceSpan};

use super::participation::{ValidationCompleteness, ValidationSourceFile};

pub(crate) fn source_files_in_project_order(source_files: &[SourceFile]) -> Vec<&SourceFile> {
    let mut ordered = source_files.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left), (right_index, right)| {
        left.path.cmp(&right.path).then(left_index.cmp(right_index))
    });
    ordered
        .into_iter()
        .map(|(_index, source_file)| source_file)
        .collect()
}

pub(crate) fn validation_source_files_in_project_order<'a>(
    source_files: &[ValidationSourceFile<'a>],
) -> Vec<ValidationSourceFile<'a>> {
    let mut ordered = source_files.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left), (right_index, right)| {
        left.source_file
            .path
            .cmp(&right.source_file.path)
            .then(left_index.cmp(right_index))
    });
    ordered
        .into_iter()
        .map(|(_index, source_file)| *source_file)
        .collect()
}

pub(super) fn collect_blocks<'a>(
    source_files: &[ValidationSourceFile<'a>],
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    let mut blocks = BTreeMap::new();
    for source_file in source_files {
        if source_file.participation.block_definitions != ValidationCompleteness::Complete {
            continue;
        }
        let source_file = source_file.source_file;
        let file_blocks = blocks
            .entry(source_file.path.as_str())
            .or_insert_with(BTreeSet::new);
        for block in &source_file.blocks {
            file_blocks.insert(block.id.as_str());
        }
    }

    blocks
}

// Invariant: 1:1 is a valid fallback source position for empty source-file sets.
#[allow(clippy::expect_used)]
pub(super) fn first_source_span(source_files: &[&SourceFile]) -> SourceSpan {
    source_files
        .iter()
        .find_map(|source_file| source_file.blocks.first().map(|block| block.span.clone()))
        .unwrap_or_else(|| {
            let path = source_files
                .first()
                .map_or_else(String::new, |source_file| source_file.path.clone());
            SourceSpan::point(
                path,
                SourcePosition::new(1, 1).expect("1:1 is a valid source position"),
            )
        })
}

pub(super) fn first_validation_source_span(
    source_files: &[ValidationSourceFile<'_>],
) -> SourceSpan {
    let source_files = source_files
        .iter()
        .map(|source_file| source_file.source_file)
        .collect::<Vec<_>>();
    first_source_span(&source_files)
}

pub(crate) fn sort_diagnostics_by_source(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.span
            .file
            .cmp(&right.span.file)
            .then(left.span.start.cmp(&right.span.start))
            .then(left.span.end.cmp(&right.span.end))
            .then(left.code.as_str().cmp(right.code.as_str()))
            .then(left.message.cmp(&right.message))
    });
}
