use recite_core::{ChoiceId, CompiledDialogue, LocaleId};
use recite_runtime::{
    DialogueContext, DialogueEvent, DialogueSession, DialogueSessionOptions, DialogueTrace,
    InterpolationValueProvider, LocaleProvider, LocaleResolution, choose, choose_with, next,
    next_with, start_scene, start_scene_with_options,
};

use crate::error::CliError;

#[derive(Clone, Copy)]
pub(crate) struct DialogueTraversalPreview<'a> {
    locale: &'a LocaleId,
    provider: &'a dyn LocaleProvider,
}

impl<'a> DialogueTraversalPreview<'a> {
    pub(crate) fn new(locale: &'a LocaleId, provider: &'a dyn LocaleProvider) -> Self {
        Self { locale, provider }
    }

    pub(crate) fn locale(&self) -> &'a LocaleId {
        self.locale
    }

    pub(crate) fn provider(&self) -> &'a dyn LocaleProvider {
        self.provider
    }
}

pub(crate) struct DialogueTraversal<'a> {
    asset: &'a CompiledDialogue,
    preview: Option<DialogueTraversalPreview<'a>>,
    values: Option<&'a dyn InterpolationValueProvider>,
    trace: Option<&'a DialogueTrace>,
}

impl<'a> DialogueTraversal<'a> {
    pub(crate) fn new(
        asset: &'a CompiledDialogue,
        preview: Option<DialogueTraversalPreview<'a>>,
    ) -> Self {
        Self {
            asset,
            preview,
            values: None,
            trace: None,
        }
    }

    pub(crate) fn with_values(mut self, values: &'a dyn InterpolationValueProvider) -> Self {
        self.values = Some(values);
        self
    }

    pub(crate) fn with_trace(mut self, trace: &'a DialogueTrace) -> Self {
        self.trace = Some(trace);
        self
    }

    pub(crate) fn start(&self, block: Option<&str>) -> Result<DialogueSession, CliError> {
        match self.preview {
            Some(preview) => Ok(start_scene_with_options(
                self.asset,
                block,
                DialogueSessionOptions::new().with_locale(preview.locale.clone()),
            )?),
            None => Ok(start_scene(self.asset, block)?),
        }
    }

    pub(crate) fn next(
        &self,
        session: &mut DialogueSession,
        context: &dyn DialogueContext,
    ) -> Result<DialogueEvent, recite_runtime::DialogueError> {
        match (self.preview, self.values) {
            (Some(preview), values) => next_with(
                self.asset,
                session,
                context,
                resolution(preview, values, self.trace),
            ),
            (None, Some(values)) => next_with(
                self.asset,
                session,
                context,
                trace_resolution(LocaleResolution::new().with_values(values), self.trace),
            ),
            (None, None) => next(self.asset, session, context),
        }
    }

    pub(crate) fn choose(
        &self,
        session: &mut DialogueSession,
        choice_id: ChoiceId,
        context: &dyn DialogueContext,
    ) -> Result<DialogueEvent, recite_runtime::DialogueError> {
        match (self.preview, self.values) {
            (Some(preview), values) => choose_with(
                self.asset,
                session,
                choice_id,
                context,
                resolution(preview, values, self.trace),
            ),
            (None, Some(values)) => choose_with(
                self.asset,
                session,
                choice_id,
                context,
                trace_resolution(LocaleResolution::new().with_values(values), self.trace),
            ),
            (None, None) => choose(self.asset, session, choice_id, context),
        }
    }
}

fn resolution<'a>(
    preview: DialogueTraversalPreview<'a>,
    values: Option<&'a dyn InterpolationValueProvider>,
    trace: Option<&'a DialogueTrace>,
) -> LocaleResolution<'a> {
    let resolution = LocaleResolution::new().with_provider(preview.provider);
    let resolution = values.map_or(resolution, |values| resolution.with_values(values));
    trace_resolution(resolution, trace)
}

fn trace_resolution<'a>(
    resolution: LocaleResolution<'a>,
    trace: Option<&'a DialogueTrace>,
) -> LocaleResolution<'a> {
    trace.map_or(resolution, |trace| resolution.with_trace(trace))
}
