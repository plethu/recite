use super::{Server, ServerError};

use super::bootstrap;

pub(super) enum RegistrationState {
    NotRequested,
    Pending,
    Registered,
}

impl RegistrationState {
    pub(super) fn new(dynamic_watched_files: bool) -> Self {
        if dynamic_watched_files {
            Self::Pending
        } else {
            Self::NotRequested
        }
    }
}

impl Server {
    pub(super) fn handle_initialized(&mut self) -> Result<(), ServerError> {
        if !matches!(self.watched_files_registration, RegistrationState::Pending) {
            return Ok(());
        }

        bootstrap::register_watched_files(self)?;
        self.watched_files_registration = RegistrationState::Registered;
        Ok(())
    }
}
