use recite_core::{ChoiceId, CompiledDialogue, LocaleId};
use recite_runtime::{
    DialogueContext, DialogueEvent, DialogueSession, DialogueSessionOptions, LocaleProvider,
    LocaleResolution, choose, choose_with, next, next_with, start_scene, start_scene_with_options,
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

    pub(crate) fn locale(&self) -> &LocaleId {
        self.locale
    }
}

pub(crate) struct DialogueTraversal<'a> {
    asset: &'a CompiledDialogue,
    preview: Option<DialogueTraversalPreview<'a>>,
}

impl<'a> DialogueTraversal<'a> {
    pub(crate) fn new(
        asset: &'a CompiledDialogue,
        preview: Option<DialogueTraversalPreview<'a>>,
    ) -> Self {
        Self { asset, preview }
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
        match self.preview {
            Some(preview) => next_with(
                self.asset,
                session,
                context,
                LocaleResolution::new().with_provider(preview.provider),
            ),
            None => next(self.asset, session, context),
        }
    }

    pub(crate) fn choose(
        &self,
        session: &mut DialogueSession,
        choice_id: ChoiceId,
        context: &dyn DialogueContext,
    ) -> Result<DialogueEvent, recite_runtime::DialogueError> {
        match self.preview {
            Some(preview) => choose_with(
                self.asset,
                session,
                choice_id,
                context,
                LocaleResolution::new().with_provider(preview.provider),
            ),
            None => choose(self.asset, session, choice_id, context),
        }
    }
}
