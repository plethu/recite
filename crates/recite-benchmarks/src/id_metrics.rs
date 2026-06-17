use std::mem::size_of;

use recite_core::{
    BlockId, ChoiceEcho, ChoiceId, CompiledChoiceEcho, CompiledDialogue, CompiledEffect,
    DivertTarget, EffectId, LineId, LocaleId, SourceFile, SpeakerId, Statement,
};
use recite_runtime::DialogueSession;
use serde::Serialize;

use crate::fixture_context::RuntimeFixture;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdStorageReport {
    pub active_storage: &'static str,
    pub string_size_bytes: usize,
    pub id_size_bytes: usize,
    pub compact_inline_capacity_bytes: usize,
    pub compiled_block_size_bytes: usize,
    pub compiled_line_size_bytes: usize,
    pub compiled_choice_size_bytes: usize,
    pub compiled_effect_size_bytes: usize,
    pub compiled_speaker_size_bytes: usize,
    pub dialogue_session_size_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdMetricSet {
    pub total: IdMetrics,
    pub by_kind: Vec<IdKindMetrics>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdKindMetrics {
    pub kind: &'static str,
    #[serde(flatten)]
    pub metrics: IdMetrics,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct IdMetrics {
    pub count: u64,
    pub text_bytes: u64,
    pub max_bytes: usize,
    pub string_heap_payload_bytes: u64,
    pub compact_inline_count: u64,
    pub compact_heap_count: u64,
    pub compact_heap_payload_bytes: u64,
}

#[must_use]
pub fn id_storage_report() -> IdStorageReport {
    IdStorageReport {
        active_storage: active_storage(),
        string_size_bytes: size_of::<String>(),
        id_size_bytes: size_of::<LineId>(),
        compact_inline_capacity_bytes: compact_inline_capacity(),
        compiled_block_size_bytes: size_of::<recite_core::CompiledBlock>(),
        compiled_line_size_bytes: size_of::<recite_core::CompiledLine>(),
        compiled_choice_size_bytes: size_of::<recite_core::CompiledChoice>(),
        compiled_effect_size_bytes: size_of::<CompiledEffect>(),
        compiled_speaker_size_bytes: size_of::<recite_core::CompiledSpeaker>(),
        dialogue_session_size_bytes: size_of::<DialogueSession>(),
    }
}

#[must_use]
pub fn source_id_metrics(files: &[SourceFile]) -> IdMetricSet {
    let mut metrics = IdMetricAccumulator::new();
    for file in files {
        collect_source_file(file, &mut metrics);
    }
    metrics.finish()
}

#[must_use]
pub fn compiled_id_metrics(dialogue: &CompiledDialogue) -> IdMetricSet {
    let mut metrics = IdMetricAccumulator::new();

    for block in &dialogue.blocks {
        metrics.add_block(&block.id);
    }
    for line in &dialogue.lines {
        metrics.add_line(&line.id);
    }
    for choice in &dialogue.choices {
        metrics.add_choice(&choice.id);
        if let CompiledChoiceEcho::ExplicitLine(id) = &choice.echo {
            metrics.add_line(id);
        }
    }
    for speaker in &dialogue.speakers {
        metrics.add_speaker(&speaker.id);
    }
    for effect in &dialogue.effects {
        metrics.add_effect(&effect.id);
    }
    for entry in dialogue.block_lookup.iter() {
        metrics.add_block(&entry.id);
    }
    for entry in dialogue.line_lookup.iter() {
        metrics.add_line(&entry.id);
    }
    for entry in dialogue.choice_lookup.iter() {
        metrics.add_choice(&entry.id);
    }

    metrics.finish()
}

#[must_use]
pub fn runtime_fixture_id_metrics(fixture: &RuntimeFixture) -> IdMetricSet {
    let mut metrics = IdMetricAccumulator::new();
    metrics.add_locale(fixture.locale_ref());
    for choice in fixture.choice_ids() {
        metrics.add_choice(choice);
    }
    metrics.finish()
}

#[must_use]
pub const fn active_storage() -> &'static str {
    "compact_str"
}

#[must_use]
pub const fn compact_inline_capacity() -> usize {
    size_of::<String>()
}

fn collect_source_file(file: &SourceFile, metrics: &mut IdMetricAccumulator) {
    for block in &file.blocks {
        metrics.add_block(&block.id);
        if let Some(speaker) = &block.default_speaker {
            metrics.add_speaker(speaker);
        }

        block.visit_statements_depth_first(&mut |statement| {
            collect_source_statement(statement, metrics);
        });
    }
}

fn collect_source_statement(statement: &Statement, metrics: &mut IdMetricAccumulator) {
    match statement {
        Statement::Line(line) => {
            if let Some(id) = &line.id {
                metrics.add_line(id);
            }
            if let Some(speaker) = &line.speaker {
                metrics.add_speaker(speaker);
            }
        }
        Statement::Choice(choice) => {
            if let Some(id) = &choice.id {
                metrics.add_choice(id);
            }
            if let ChoiceEcho::Line(id) = &choice.echo {
                metrics.add_line(id);
            }
            if let Some(target) = &choice.target {
                collect_divert_target(&target.target, metrics);
            }
        }
        Statement::Divert(divert) => collect_divert_target(&divert.target, metrics),
        Statement::If(_) | Statement::Match(_) | Statement::Effect(_) | Statement::Comment(_) => {}
    }
}

fn collect_divert_target(target: &DivertTarget, metrics: &mut IdMetricAccumulator) {
    if let DivertTarget::Block(reference) = target {
        metrics.add_block(&reference.block_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum IdKind {
    Block,
    Choice,
    Effect,
    Line,
    Locale,
    Speaker,
}

impl IdKind {
    const ALL: [Self; 6] = [
        Self::Block,
        Self::Choice,
        Self::Effect,
        Self::Line,
        Self::Locale,
        Self::Speaker,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Choice => "choice",
            Self::Effect => "effect",
            Self::Line => "line",
            Self::Locale => "locale",
            Self::Speaker => "speaker",
        }
    }
}

#[derive(Clone, Debug, Default)]
struct IdMetricAccumulator {
    block: IdMetrics,
    choice: IdMetrics,
    effect: IdMetrics,
    line: IdMetrics,
    locale: IdMetrics,
    speaker: IdMetrics,
}

impl IdMetricAccumulator {
    const fn new() -> Self {
        Self {
            block: IdMetrics::new(),
            choice: IdMetrics::new(),
            effect: IdMetrics::new(),
            line: IdMetrics::new(),
            locale: IdMetrics::new(),
            speaker: IdMetrics::new(),
        }
    }

    fn add_block(&mut self, id: &BlockId) {
        self.add(IdKind::Block, id.as_str());
    }

    fn add_choice(&mut self, id: &ChoiceId) {
        self.add(IdKind::Choice, id.as_str());
    }

    fn add_effect(&mut self, id: &EffectId) {
        self.add(IdKind::Effect, id.as_str());
    }

    fn add_line(&mut self, id: &LineId) {
        self.add(IdKind::Line, id.as_str());
    }

    fn add_locale(&mut self, id: &LocaleId) {
        self.add(IdKind::Locale, id.as_str());
    }

    fn add_speaker(&mut self, id: &SpeakerId) {
        self.add(IdKind::Speaker, id.as_str());
    }

    fn add(&mut self, kind: IdKind, value: &str) {
        self.metrics_mut(kind).add(value);
    }

    fn finish(self) -> IdMetricSet {
        let by_kind = IdKind::ALL
            .into_iter()
            .map(|kind| IdKindMetrics {
                kind: kind.as_str(),
                metrics: self.metrics(kind).clone(),
            })
            .collect::<Vec<_>>();
        let total = by_kind.iter().fold(IdMetrics::new(), |mut total, kind| {
            total.merge(&kind.metrics);
            total
        });

        IdMetricSet { total, by_kind }
    }

    const fn metrics(&self, kind: IdKind) -> &IdMetrics {
        match kind {
            IdKind::Block => &self.block,
            IdKind::Choice => &self.choice,
            IdKind::Effect => &self.effect,
            IdKind::Line => &self.line,
            IdKind::Locale => &self.locale,
            IdKind::Speaker => &self.speaker,
        }
    }

    fn metrics_mut(&mut self, kind: IdKind) -> &mut IdMetrics {
        match kind {
            IdKind::Block => &mut self.block,
            IdKind::Choice => &mut self.choice,
            IdKind::Effect => &mut self.effect,
            IdKind::Line => &mut self.line,
            IdKind::Locale => &mut self.locale,
            IdKind::Speaker => &mut self.speaker,
        }
    }
}

impl IdMetrics {
    const fn new() -> Self {
        Self {
            count: 0,
            text_bytes: 0,
            max_bytes: 0,
            string_heap_payload_bytes: 0,
            compact_inline_count: 0,
            compact_heap_count: 0,
            compact_heap_payload_bytes: 0,
        }
    }

    fn add(&mut self, value: &str) {
        let bytes = value.len();
        self.count += 1;
        self.text_bytes += bytes as u64;
        self.max_bytes = self.max_bytes.max(bytes);
        self.string_heap_payload_bytes += bytes as u64;
        if bytes <= compact_inline_capacity() {
            self.compact_inline_count += 1;
        } else {
            self.compact_heap_count += 1;
            self.compact_heap_payload_bytes += bytes as u64;
        }
    }

    fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.text_bytes += other.text_bytes;
        self.max_bytes = self.max_bytes.max(other.max_bytes);
        self.string_heap_payload_bytes += other.string_heap_payload_bytes;
        self.compact_inline_count += other.compact_inline_count;
        self.compact_heap_count += other.compact_heap_count;
        self.compact_heap_payload_bytes += other.compact_heap_payload_bytes;
    }
}
