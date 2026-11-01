//! Baseline reload also reconciles project authority before analysis refresh.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAccepted {
    Draft,
    Reloaded,
    ContextChanged,
}
impl ProjectFiles {
    pub fn accept_settings(
        &mut self,
        receipt: &mut SettingsReceipt,
    ) -> Result<SettingsAccepted, FileError> {
        let session = self.settings.as_mut().ok_or(FileError::SettingsChanged)?;
        let reloading = matches!(&session.activity, Activity::Reloading(identity) if Arc::ptr_eq(identity,&receipt.identity));
        let changed = reloading
            && receipt
                .result
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .is_some_and(|next| {
                    session.requires_reload()
                        || next.owner.last_known().source() != session.baseline()
                });
        let accepted = session.accept(receipt);
        if reloading {
            self.set_saving(false);
        }
        if reloading
            && accepted.is_err()
            && matches!(receipt.observation, BaselineObservation::Attempted)
        {
            self.quarantine_settings(receipt)?;
        }
        accepted?;
        if !reloading {
            return Ok(SettingsAccepted::Draft);
        }
        let session = self.settings.as_ref().ok_or(FileError::SettingsChanged)?;
        self.manifest.refresh(session.baseline().to_owned());
        if !changed {
            return Ok(if self.configuration_refresh_required() {
                SettingsAccepted::ContextChanged
            } else {
                SettingsAccepted::Reloaded
            });
        }
        if self.declarations.as_ref().is_some_and(|declarations| {
            !declarations.matches_schema(session.snapshot.schema.as_deref())
        }) {
            receipt.retired_declarations = self.declarations.take();
        }
        self.configuration_change = Some(ConfigurationChange(receipt.identity.clone()));
        Ok(SettingsAccepted::ContextChanged)
    }
    fn quarantine_settings(&mut self, receipt: &mut SettingsReceipt) -> Result<(), FileError> {
        let session = self.settings.as_mut().ok_or(FileError::SettingsChanged)?;
        let next = Arc::new(Snapshot {
            owner: Baseline::Unconfirmed(session.snapshot.owner.last_known().clone()),
            schema: session.snapshot.schema.clone(),
            artifact: session.snapshot.artifact.clone(),
            review: session.snapshot.review.clone(),
        });
        receipt.retired = Some(std::mem::replace(&mut session.snapshot, next));
        receipt.retired_declarations = self.declarations.take();
        self.configuration_change = Some(ConfigurationChange(receipt.identity.clone()));
        Ok(())
    }
}
