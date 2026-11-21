//! Author-facing prose regions for spelling and grammar tooling.
//!
//! The parser owns which source bytes are prose. Dialogue line bodies, plural
//! forms and choice bodies are prose; headers, stable IDs, speakers, metadata
//! keys and symbols, conditions, effects, diverts and comments are not. Quoted
//! metadata string values are reported separately so a project can opt in to
//! checking keys such as `caption`. Regions keep exact file byte ranges and
//! survive the recovery that incomplete files receive while they are edited.

use std::ops::Range;

use recite_core::SourceSpan;
use recite_core::ast::Statement;

use crate::source::{LogicalLine, LogicalLines};

mod text;

pub use text::{ProsePiece, ProsePieceKind, ProseText};

/// What kind of author-facing text a [`ProseRegion`] holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ProseRegionKind {
    /// The body of a `>` dialogue line.
    Line,
    /// The `|` plural form of a dialogue line.
    PluralLine,
    /// The body of a `?` choice.
    Choice,
    /// A quoted metadata string value on a block, line or choice header.
    Metadata,
}

impl ProseRegionKind {
    /// Stable lowercase name used by structured tooling output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::PluralLine => "plural_line",
            Self::Choice => "choice",
            Self::Metadata => "metadata",
        }
    }
}

/// One physical source line of a prose region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProseLine {
    bytes: Range<usize>,
    paragraph_start: bool,
}

impl ProseLine {
    /// File byte range of the line's prose, without indentation or newline.
    #[must_use]
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }

    /// Whether a blank line (or the region start) precedes this line.
    #[must_use]
    pub const fn paragraph_start(&self) -> bool {
        self.paragraph_start
    }
}

/// A run of author-facing text and its exact source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProseRegion {
    kind: ProseRegionKind,
    stable_id: Option<String>,
    speaker: Option<String>,
    metadata_key: Option<String>,
    lines: Vec<ProseLine>,
}

impl ProseRegion {
    #[must_use]
    pub const fn kind(&self) -> ProseRegionKind {
        self.kind
    }

    /// The owning line or choice ID, or the block name for block metadata.
    #[must_use]
    pub fn stable_id(&self) -> Option<&str> {
        self.stable_id.as_deref()
    }

    /// The line speaker, or the block default speaker for lines without one.
    /// Choices have no speaker.
    #[must_use]
    pub fn speaker(&self) -> Option<&str> {
        self.speaker.as_deref()
    }

    /// The metadata key of a [`ProseRegionKind::Metadata`] region.
    #[must_use]
    pub fn metadata_key(&self) -> Option<&str> {
        self.metadata_key.as_deref()
    }

    /// Source lines in order; blank lines between paragraphs are omitted.
    #[must_use]
    pub fn lines(&self) -> &[ProseLine] {
        &self.lines
    }

    /// File byte range from the first prose byte to the last.
    #[must_use]
    pub fn bytes(&self) -> Range<usize> {
        let start = self.lines.first().map_or(0, |line| line.bytes.start);
        let end = self.lines.last().map_or(start, |line| line.bytes.end);
        start..end
    }

    /// Checkable text with inline markup, placeholders and escapes masked.
    /// `source` must be the text this region was extracted from.
    #[must_use]
    pub fn text(&self, source: &str) -> ProseText {
        text::build(self, source)
    }
}

/// Extract every prose region from a source file in source order.
///
/// Extraction uses the parser's own lowering and recovery, so it works on
/// incomplete files: a region is reported for each line or choice the parser
/// recognises, even when the file also has syntax errors.
#[must_use]
pub fn prose_regions(source: &str) -> Vec<ProseRegion> {
    let lowered = crate::lower_source_file("prose", source);
    let lines = IndexedLines::new(source);
    let mut regions = Vec::new();
    for block in &lowered.source_file.blocks {
        let block_owner = Owner {
            stable_id: Some(block.id.as_str().to_owned()),
            speaker: None,
        };
        regions.extend(metadata_regions(&lines, &block.span, &block_owner));
        let default_speaker = block
            .default_speaker
            .as_ref()
            .map(|speaker| speaker.as_str());
        for statement in &block.statements {
            statement.visit_depth_first(&mut |statement| {
                statement_regions(&lines, default_speaker, statement, &mut regions);
            });
        }
    }
    regions.sort_by_key(|region| region.bytes().start);
    regions
}

/// The statement that owns a region.
struct Owner {
    stable_id: Option<String>,
    speaker: Option<String>,
}

impl Owner {
    fn region(&self, kind: ProseRegionKind, lines: Vec<ProseLine>) -> ProseRegion {
        ProseRegion {
            kind,
            stable_id: self.stable_id.clone(),
            speaker: self.speaker.clone(),
            metadata_key: None,
            lines,
        }
    }
}

fn statement_regions(
    lines: &IndexedLines<'_>,
    default_speaker: Option<&str>,
    statement: &Statement,
    regions: &mut Vec<ProseRegion>,
) {
    match statement {
        Statement::Line(line) => {
            let owner = Owner {
                stable_id: line.id.as_ref().map(|id| id.as_str().to_owned()),
                speaker: line
                    .speaker
                    .as_ref()
                    .map(|speaker| speaker.as_str())
                    .or(default_speaker)
                    .map(ToOwned::to_owned),
            };
            regions.extend(metadata_regions(lines, &line.span, &owner));
            regions.extend(body_region(
                lines,
                &owner,
                ProseRegionKind::Line,
                &line.source_text.text,
                &line.source_text.span,
            ));
            if let Some(plural) = &line.plural_source_text {
                regions.extend(plural_region(lines, &owner, &plural.text, &plural.span));
            }
        }
        Statement::Choice(choice) => {
            let owner = Owner {
                stable_id: choice.id.as_ref().map(|id| id.as_str().to_owned()),
                speaker: None,
            };
            regions.extend(metadata_regions(lines, &choice.span, &owner));
            regions.extend(body_region(
                lines,
                &owner,
                ProseRegionKind::Choice,
                &choice.source_text.text,
                &choice.source_text.span,
            ));
        }
        Statement::Divert(_)
        | Statement::If(_)
        | Statement::Match(_)
        | Statement::Effect(_)
        | Statement::Comment(_) => {}
    }
}

/// Map lowered body text back to the source lines it was joined from. The
/// lowering keeps each body line's content after the body indentation and
/// joins lines with `\n`, inserting an empty piece for each blank line, so
/// matching indentation and content in order recovers the exact bytes.
fn body_region(
    lines: &IndexedLines<'_>,
    owner: &Owner,
    kind: ProseRegionKind,
    text: &str,
    start: &SourceSpan,
) -> Option<ProseRegion> {
    if text.is_empty() {
        return None;
    }
    let indent = usize::try_from(start.start.column()).ok()?.checked_sub(1)?;
    let mut index = usize::try_from(start.start.line()).ok()?.checked_sub(1)?;
    let mut paragraph_start = true;
    let mut prose_lines = Vec::new();
    for piece in text.split('\n') {
        if piece.is_empty() {
            paragraph_start = true;
            continue;
        }
        while let Some((offset, line)) = lines.get(index) {
            index += 1;
            if line.indent_len() == indent && line.trimmed_content() == piece {
                prose_lines.push(ProseLine {
                    bytes: offset + indent..offset + line.text.len(),
                    paragraph_start,
                });
                paragraph_start = false;
                break;
            }
        }
    }
    (!prose_lines.is_empty()).then(|| owner.region(kind, prose_lines))
}

fn plural_region(
    lines: &IndexedLines<'_>,
    owner: &Owner,
    text: &str,
    span: &SourceSpan,
) -> Option<ProseRegion> {
    if text.is_empty() {
        return None;
    }
    let (offset, line) = lines.get(usize::try_from(span.start.line()).ok()?.checked_sub(1)?)?;
    let column = usize::try_from(span.start.column()).ok()?.checked_sub(1)?;
    let end = column.checked_add(text.len())?;
    (line.text.get(column..end) == Some(text)).then(|| {
        owner.region(
            ProseRegionKind::PluralLine,
            vec![ProseLine {
                bytes: offset + column..offset + end,
                paragraph_start: true,
            }],
        )
    })
}

/// Quoted string values in one statement header, using the parser's own
/// header field scanner so brackets, conditions and escapes stay consistent.
fn metadata_regions(
    lines: &IndexedLines<'_>,
    header: &SourceSpan,
    owner: &Owner,
) -> Vec<ProseRegion> {
    let Some((offset, line)) = usize::try_from(header.start.line())
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| lines.get(index))
    else {
        return Vec::new();
    };
    crate::metadata_assignments(line.text)
        .into_iter()
        .filter(|assignment| {
            assignment.value.len() > 2
                && assignment.value.starts_with('"')
                && assignment.value.ends_with('"')
                && crate::parse_metadata_value(assignment.value).is_some()
        })
        .map(|assignment| ProseRegion {
            metadata_key: Some(assignment.key.to_owned()),
            ..owner.region(
                ProseRegionKind::Metadata,
                vec![ProseLine {
                    bytes: offset + assignment.value_start + 1..offset + assignment.end - 1,
                    paragraph_start: true,
                }],
            )
        })
        .collect()
}

struct IndexedLines<'a> {
    lines: Vec<(usize, LogicalLine<'a>)>,
}

impl<'a> IndexedLines<'a> {
    fn new(source: &'a str) -> Self {
        let mut offset = 0;
        let lines = LogicalLines::new(source)
            .map(|line| {
                let start = offset;
                offset += line.text.len() + line.newline.len();
                (start, line)
            })
            .collect();
        Self { lines }
    }

    fn get(&self, index: usize) -> Option<(usize, LogicalLine<'a>)> {
        self.lines.get(index).copied()
    }
}
