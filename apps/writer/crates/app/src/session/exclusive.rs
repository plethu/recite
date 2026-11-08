//! One worker owns the real project; the mounted surface has reading authority only.
use super::*;
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::FrozenWorkbench;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

struct Claim {
    cancel: CancellationToken,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryReason {
    RetryCheckpoint,
    ReopenRequired,
}
#[derive(Clone)]
pub struct ExclusiveTicket(Arc<Claim>);
pub struct FrozenProject {
    root: PathBuf,
    model: FrozenWorkbench,
    pub(super) current_dirty: bool,
    pub(super) dirty: bool,
}
pub struct Checkout {
    pub(crate) ticket: ExclusiveTicket,
    pub project: Option<Box<ProjectSession>>,
    pub retired: Option<Box<FrozenProject>>,
}
impl ExclusiveTicket {
    pub fn cancel(&self) {
        self.0.cancel.interrupt();
    }
    fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl WorkControl for ExclusiveTicket {
    fn checkpoint(&self) -> Result<(), recite_compiler::authoring::Interrupted> {
        self.0.cancel.checkpoint()
    }
}
impl FrozenProject {
    pub fn model(&self) -> &Workbench {
        self.model.model()
    }
}
impl Session {
    pub fn recovery_reason(&self) -> Option<RecoveryReason> {
        match self {
            Self::Recovering { reason, .. } => Some(*reason),
            _ => None,
        }
    }
    pub fn read_only(&self) -> bool {
        matches!(self, Self::Working { .. } | Self::Recovering { .. })
    }
    pub fn checked_out(&self) -> bool {
        matches!(self, Self::Working { .. })
    }
    pub fn project_root(&self) -> Option<&Path> {
        match self {
            Self::Working { frozen, .. } => Some(&frozen.root),
            _ => self.project().map(ProjectSession::root),
        }
    }
    pub(crate) fn cancel_exclusive(&self) {
        if let Self::Working { ticket, .. } = self {
            ticket.cancel();
        }
    }
    pub fn checkout(&mut self) -> Result<Checkout, FileError> {
        if self.recovery_reason() == Some(RecoveryReason::ReopenRequired) {
            return Err(FileError::ProjectEditRecoveryRestart);
        }
        let (Self::Project(project) | Self::Recovering { project, .. }) = self else {
            return Err(FileError::ProjectActionPending);
        };
        let frozen = Box::new(FrozenProject {
            root: project.root().to_owned(),
            model: project.current().model().freeze(),
            current_dirty: project.current().dirty(),
            dirty: project.dirty(),
        });
        let ticket = ExclusiveTicket(Arc::new(Claim {
            cancel: CancellationToken::new(),
        }));
        let previous = std::mem::replace(
            self,
            Self::Working {
                ticket: ticket.clone(),
                frozen,
            },
        );
        match previous {
            Self::Project(project) | Self::Recovering { project, .. } => Ok(Checkout {
                ticket,
                project: Some(project),
                retired: None,
            }),
            previous => {
                *self = previous;
                Err(FileError::ProjectActionPending)
            }
        }
    }
    pub fn restore_checkout(
        &mut self,
        checkout: &mut Checkout,
        recovering: Option<RecoveryReason>,
    ) -> Result<(), FileError> {
        let Self::Working { ticket, frozen } = self else {
            return Err(FileError::ProjectActionPending);
        };
        if !ticket.matches(&checkout.ticket)
            || checkout
                .project
                .as_ref()
                .is_none_or(|project| project.root() != frozen.root)
        {
            return Err(FileError::ProjectActionPending);
        }
        let project = checkout
            .project
            .take()
            .ok_or(FileError::ProjectActionPending)?;
        let restored = match recovering {
            Some(reason) => Self::Recovering { project, reason },
            None => Self::Project(project),
        };
        if let Self::Working { frozen, .. } = std::mem::replace(self, restored) {
            checkout.retired = Some(frozen);
        }
        Ok(())
    }
}
