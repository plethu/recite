use super::{Server, ServerError, updates::Update};
use lsp_server::Notification;
use std::sync::Arc;

impl Server {
    pub(super) fn notification(&mut self, notification: Notification) -> Result<(), ServerError> {
        match notification.method.as_str() {
            "$/cancelRequest" => {
                self.cancel(notification.params);
                return Ok(());
            }
            "exit" => {
                if !self.shutdown_requested {
                    return Err(ServerError::ExitWithoutShutdown);
                }
                self.exit_received = true;
                return Ok(());
            }
            _ => {}
        }
        if self.shutdown_requested {
            return Ok(());
        }
        let Some(mut update) = Update::parse(notification) else {
            return Ok(());
        };
        if !self.documents.accept(&mut update) {
            return Ok(());
        }
        let scope = update
            .changed_uri()
            .and_then(|uri| self.known_scope(uri))
            .map(str::to_owned);
        self.epochs
            .advance(scope.as_deref())
            .ok_or(ServerError::SequenceExhausted)?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ServerError::SequenceExhausted)?;
        self.invalidate();
        if let Some(uri) = update.changed_uri() {
            // Only coalesce inside a pure-change segment. Lifecycle and filesystem
            // notifications are ordering barriers, even when they name other files.
            if let Some(index) = self
                .updates
                .iter()
                .rposition(|(_, old)| old.changed_uri().is_none())
            {
                let remove = self
                    .updates
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .find(|(_, (_, old))| old.changed_uri() == Some(uri))
                    .map(|(index, _)| index);
                if let Some(index) = remove {
                    self.updates.remove(index);
                }
            } else {
                self.updates
                    .retain(|(_, old)| old.changed_uri() != Some(uri));
            }
        }
        if self.updates.len() >= 256 {
            return Err(ServerError::InputCapacity);
        }
        self.updates.push_back((self.revision, Arc::new(update)));
        // Do not repeatedly restart bootstrap or mixed-partition work. A hot
        // editor may supersede its own exclusive job, but cannot prevent a
        // sibling's already accepted revision from completing.
        if let Some(partition) = &self.analysis_partition
            && self.exclusive_update_partition().as_ref() == Some(partition)
            && let Some(control) = &self.analyzing
        {
            control.interrupt();
        }
        Ok(())
    }
}
