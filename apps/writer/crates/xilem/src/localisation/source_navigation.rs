//! A catalogue action carries its scope through Apply and final navigation publication.
use super::{Destination, EntryKey, Localisation, Screen, Writer};
use crate::navigation::Target;
use recite_core::LocaleId;
use recite_writer_model::CatalogueRevision;
use recite_writer_session::{
    localisation::{navigation::Destination as SourceDestination, target::document_language},
    location::{Location, Screen as LocationScreen},
};

#[derive(Clone, Copy)]
pub(super) enum SourceAction {
    Show,
    Preview,
}

#[derive(Clone)]
enum Mode {
    Show,
    Preview {
        locale: Option<LocaleId>,
        variant: String,
    },
}

#[derive(Clone)]
pub(crate) struct SourceIntent {
    key: EntryKey,
    revision: CatalogueRevision,
    location: Location,
    mode: Mode,
}

impl SourceIntent {
    fn capture(writer: &Writer, key: &EntryKey, action: SourceAction) -> Result<Self, String> {
        let state = &writer.localisation;
        let owner = state
            .slot
            .owner()
            .ok_or("Finish the catalogue action first.")?;
        if state.slot.read_only() || owner.catalogue.saving {
            return Err("Finish the catalogue action first.".into());
        }
        let entry = owner
            .catalogue
            .document
            .entry(key.id)
            .ok_or("Choose a translation entry.")?;
        // Resolve against the current source, rather than the queue's older presentation.
        let destination = SourceDestination::resolve(
            entry,
            &writer
                .session
                .model()
                .document()
                .cached_passages()
                .unwrap_or_default(),
        )
        .ok_or("This translation has no source location.")?;
        let mode = match action {
            SourceAction::Show => Mode::Show,
            SourceAction::Preview => Mode::Preview {
                locale: document_language(&owner.catalogue.document)?,
                variant: entry.variant().unwrap_or_default().into(),
            },
        };
        let intent = Self {
            key: key.clone(),
            revision: owner.catalogue.revision(),
            location: Location {
                screen: if matches!(mode, Mode::Preview { .. }) {
                    LocationScreen::Preview
                } else {
                    LocationScreen::Write
                },
                document: destination
                    .document
                    .unwrap_or_else(|| writer.session.model().document().key().to_string()),
                beat: Some(destination.beat),
                passage: Some(destination.anchor),
                view: Some(recite_config::WriterView::Script),
                ..Location::default()
            },
            mode,
        };
        intent.check(writer)?;
        Ok(intent)
    }

    pub(crate) fn location(&self) -> &Location {
        &self.location
    }

    pub(crate) fn check(&self, writer: &Writer) -> Result<(), String> {
        let Localisation {
            slot,
            entry,
            destination,
            ..
        } = &writer.localisation;
        if writer.workspaces.screen == Screen::Localisation
            && matches!(destination, Destination::Entry)
            && entry.as_ref().is_some_and(|entry| entry.key == self.key)
            && slot.owner().is_some_and(|owner| {
                owner.identity == self.key.catalogue && owner.catalogue.revision() == self.revision
            })
            && !slot.read_only()
        {
            Ok(())
        } else {
            Err("The translation changed while source navigation was preparing. Your work stays open.".into())
        }
    }

    pub(crate) fn complete(&self, writer: &mut Writer) {
        if let Mode::Preview { locale, variant } = &self.mode {
            writer.trial.settings.locale = locale
                .as_ref()
                .map_or_else(String::new, ToString::to_string);
            writer.trial.settings.variant = variant.clone();
        }
    }
}

impl Writer {
    pub(super) fn open_translation_source(&mut self, key: &EntryKey, action: SourceAction) {
        match SourceIntent::capture(self, key, action) {
            Ok(intent) => self.navigate(Target::CatalogueSource(intent)),
            Err(error) => self.catalogue_feedback(error),
        }
    }
}

#[cfg(test)]
mod tests;
