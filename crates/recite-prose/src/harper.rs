//! The embedded Harper engine: offline, local and Apache-2.0 licensed.

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;

use harper_core::linting::{Lint, LintGroup, LintKind, Suggestion};
use harper_core::parsers::{Parser, PlainEnglish};
use harper_core::spell::{FstDictionary, MergedDictionary, MutableDictionary};
use harper_core::{Dialect, DictWordMetadata, Document, Span, Token, TokenKind};
use recite_parser::{ProsePieceKind, ProseText};

use crate::{EngineFinding, ProseEngine, ProseFindingKind, ProseLanguage};

/// Harper with the curated English rules, a dialect and project words.
pub struct HarperEngine {
    dictionary: Arc<MergedDictionary>,
    rules: LintGroup,
}

impl HarperEngine {
    #[must_use]
    pub fn new<'a>(
        language: ProseLanguage,
        words: impl IntoIterator<Item = &'a str>,
        ignored_rules: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut project = MutableDictionary::new();
        for word in words {
            project.append_word_str(word, DictWordMetadata::default());
        }
        let mut dictionary = MergedDictionary::new();
        dictionary.add_dictionary(FstDictionary::curated());
        dictionary.add_dictionary(Arc::new(project));
        let dictionary = Arc::new(dictionary);
        let mut rules = LintGroup::new_curated(dictionary.clone(), dialect(language));
        for rule in ignored_rules {
            rules.config.set_rule_enabled(rule, false);
        }
        Self { dictionary, rules }
    }
}

impl ProseEngine for HarperEngine {
    fn check(&mut self, text: &ProseText) -> Vec<EngineFinding> {
        let chars = CharIndex::new(text.as_str());
        let parser = MaskedPlainEnglish {
            unlintable: text
                .pieces()
                .iter()
                .filter(|piece| piece.kind() == ProsePieceKind::Placeholder)
                .map(|piece| chars.char_range(piece.text_range()))
                .collect(),
        };
        let document = Document::new(text.as_str(), &parser, self.dictionary.as_ref());
        let mut lints = self.rules.organized_lints(&document);
        harper_core::remove_overlaps_map(&mut lints);
        let mut findings = Vec::new();
        for (rule, lints) in lints {
            findings.extend(
                lints
                    .into_iter()
                    .map(|lint| finding(&chars, rule.clone(), lint)),
            );
        }
        findings
    }
}

fn finding(chars: &CharIndex, rule: String, lint: Lint) -> EngineFinding {
    let range = chars.byte_range(lint.span.start..lint.span.end);
    let suggestions = lint
        .suggestions
        .iter()
        .map(|suggestion| match suggestion {
            Suggestion::ReplaceWith(replacement) => {
                (range.clone(), replacement.iter().collect::<String>())
            }
            Suggestion::InsertAfter(insertion) => {
                (range.end..range.end, insertion.iter().collect::<String>())
            }
            Suggestion::Remove => (range.clone(), String::new()),
        })
        .collect();
    EngineFinding {
        kind: finding_kind(lint.lint_kind),
        rule,
        message: lint.message,
        range,
        suggestions,
    }
}

const fn finding_kind(kind: LintKind) -> ProseFindingKind {
    match kind {
        LintKind::Spelling => ProseFindingKind::Spelling,
        LintKind::Enhancement
        | LintKind::Formatting
        | LintKind::Readability
        | LintKind::Redundancy
        | LintKind::Style => ProseFindingKind::Style,
        _ => ProseFindingKind::Grammar,
    }
}

const fn dialect(language: ProseLanguage) -> Dialect {
    match language {
        ProseLanguage::EnglishGb => Dialect::British,
        ProseLanguage::EnglishAu => Dialect::Australian,
        ProseLanguage::EnglishCa => Dialect::Canadian,
        ProseLanguage::EnglishIn => Dialect::Indian,
        ProseLanguage::EnglishUs => Dialect::American,
    }
}

/// Plain English, except placeholders become unlintable tokens. Rules then
/// neither check them as words nor use them as context, so `a {item}` is
/// not an article error whatever the caller substitutes.
struct MaskedPlainEnglish {
    unlintable: BTreeSet<(usize, usize)>,
}

impl Parser for MaskedPlainEnglish {
    fn parse(&self, source: &[char]) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut cursor = 0;
        for &(start, end) in &self.unlintable {
            push_plain(&mut tokens, source, cursor, start);
            tokens.push(Token::new(Span::new(start, end), TokenKind::Unlintable));
            cursor = end;
        }
        push_plain(&mut tokens, source, cursor, source.len());
        tokens
    }
}

fn push_plain(tokens: &mut Vec<Token>, source: &[char], start: usize, end: usize) {
    if start >= end {
        return;
    }
    tokens.extend(
        PlainEnglish
            .parse(&source[start..end])
            .into_iter()
            .map(|mut token| {
                token.span.push_by(start);
                token
            }),
    );
}

/// Conversion between the engine's character offsets and checked-text bytes.
struct CharIndex {
    starts: Vec<usize>,
}

impl CharIndex {
    fn new(text: &str) -> Self {
        let mut starts = text
            .char_indices()
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        starts.push(text.len());
        Self { starts }
    }

    fn byte(&self, char_index: usize) -> usize {
        self.starts
            .get(char_index)
            .or(self.starts.last())
            .copied()
            .unwrap_or_default()
    }

    fn byte_range(&self, chars: Range<usize>) -> Range<usize> {
        self.byte(chars.start)..self.byte(chars.end)
    }

    fn char_range(&self, bytes: Range<usize>) -> (usize, usize) {
        let index = |byte: usize| self.starts.partition_point(|start| *start < byte);
        (index(bytes.start), index(bytes.end))
    }
}
