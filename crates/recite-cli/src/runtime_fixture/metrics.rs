use recite_runtime::{DialogueEffectMode, LocaleProvider, TextDomain, encode_session_messagepack};

use crate::error::CliError;

use super::trace::{TraceEffectCounts, TraceMetrics};

pub(super) fn record_session_size(
    metrics: Option<&mut RuntimeMetricsCollector>,
    session: &recite_runtime::DialogueSession,
) -> Result<(), CliError> {
    let Some(metrics) = metrics else {
        return Ok(());
    };
    metrics.max_serialized_session_size_bytes = metrics
        .max_serialized_session_size_bytes
        .max(encode_session_messagepack(session)?.len());
    Ok(())
}

#[derive(Default)]
pub(super) struct RuntimeMetricsCollector {
    pub(super) line_count: usize,
    pub(super) prompt_count: usize,
    pub(super) choice_count: usize,
    pub(super) condition_evaluation_count: usize,
    pub(super) effect_count: TraceEffectCounts,
    pub(super) max_serialized_session_size_bytes: usize,
}

impl RuntimeMetricsCollector {
    pub(super) fn record_effect(&mut self, mode: DialogueEffectMode) {
        match mode {
            DialogueEffectMode::Deferred => self.effect_count.deferred += 1,
            DialogueEffectMode::Immediate => self.effect_count.immediate += 1,
            DialogueEffectMode::Blocking => self.effect_count.blocking += 1,
        }
    }

    pub(super) fn finish(
        self,
        event_count: usize,
        localization_lookup_count: usize,
        elapsed_traversal_time_ns: u128,
    ) -> TraceMetrics {
        TraceMetrics {
            event_count,
            line_count: self.line_count,
            prompt_count: self.prompt_count,
            choice_count: self.choice_count,
            condition_evaluation_count: self.condition_evaluation_count,
            effect_count: self.effect_count,
            localization_lookup_count,
            elapsed_traversal_time_ns,
            max_serialized_session_size_bytes: self.max_serialized_session_size_bytes,
        }
    }
}

pub(super) struct CountingLocaleProvider<'a> {
    provider: &'a dyn LocaleProvider,
    lookup_count: std::cell::Cell<usize>,
}

impl<'a> CountingLocaleProvider<'a> {
    pub(super) fn new(provider: &'a dyn LocaleProvider) -> Self {
        Self {
            provider,
            lookup_count: std::cell::Cell::new(0),
        }
    }

    pub(super) fn lookup_count(&self) -> usize {
        self.lookup_count.get()
    }
}

impl LocaleProvider for CountingLocaleProvider<'_> {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &recite_core::LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, recite_runtime::LocaleError> {
        self.lookup_count.set(self.lookup_count.get() + 1);
        self.provider
            .lookup(id, source_text, domain, locale, variant)
    }

    fn resolve_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &recite_core::LocaleId,
        variant: Option<&str>,
    ) -> Result<recite_runtime::PluralResolution, recite_runtime::LocaleError> {
        self.lookup_count.set(self.lookup_count.get() + 1);
        self.provider.resolve_plural(
            id,
            source_singular,
            source_plural,
            count,
            domain,
            locale,
            variant,
        )
    }
}
