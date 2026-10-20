//! Owned requests run without borrowing a frontend or its live document.
use super::super::ordered::Write;
use crate::{
    declarations::{DeclarationSave, DeclarationSaveRequest, recovery::DraftSnapshot},
    project::{
        FileError, PreparedRefresh, RefreshRequest,
        manifest::{ManifestOutcome, ManifestSave},
    },
    recovery::{Recovery, worker::Durability},
};
use recite_writer_model::{EditRequest, PreparedEdit, WorkbenchError};

pub enum Work {
    Edit(EditRequest),
    RetireEdit(Box<PreparedEdit>),
    Write(Write),
    RetireWrite(Write),
    Recovery(Durability<Recovery>),
    Manifest(ManifestSave),
    PrepareDeclaration(Box<DeclarationSaveRequest>),
    DeclarationSource(Box<DeclarationSave>),
    DeclarationRecovery(Box<DeclarationSave>, Durability<DraftSnapshot>),
    DeclarationOutput(Box<DeclarationSave>),
    RetireDeclaration(Box<DeclarationSave>),
    Refresh(Box<RefreshRequest>),
    RetireRefresh(Box<PreparedRefresh>),
}
pub enum Receipt {
    Edit(Result<Box<PreparedEdit>, WorkbenchError>),
    Written(Write, Result<(), FileError>),
    Recovered(Result<(), FileError>),
    Manifest(ManifestOutcome),
    DeclarationPrepared(Result<Box<DeclarationSave>, FileError>),
    DeclarationSource(Box<DeclarationSave>, Result<(), FileError>),
    DeclarationRecovered(Box<DeclarationSave>, Result<(), FileError>),
    DeclarationOutput(Box<DeclarationSave>, Result<(), FileError>),
    Refreshed(Result<Box<PreparedRefresh>, FileError>),
    Retired,
}
impl Work {
    pub fn execute(self) -> Receipt {
        match self {
            Self::Edit(request) => Receipt::Edit(
                request
                    .prepare(&recite_compiler::authoring::CancellationToken::new())
                    .map(Box::new),
            ),
            Self::RetireEdit(owner) => {
                drop(owner);
                Receipt::Retired
            }
            Self::Write(mut write) => {
                let result = write.execute();
                Receipt::Written(write, result)
            }
            Self::RetireWrite(owner) => {
                drop(owner);
                Receipt::Retired
            }
            Self::Recovery(ticket) => Receipt::Recovered(ticket.wait()),
            Self::Manifest(request) => Receipt::Manifest(request.execute()),
            Self::PrepareDeclaration(request) => {
                Receipt::DeclarationPrepared(request.prepare().map(Box::new))
            }
            Self::DeclarationSource(request) => {
                let result = request.write_source();
                Receipt::DeclarationSource(request, result)
            }
            Self::DeclarationRecovery(request, ticket) => {
                Receipt::DeclarationRecovered(request, ticket.wait())
            }
            Self::DeclarationOutput(request) => {
                let result = request.write_output();
                Receipt::DeclarationOutput(request, result)
            }
            Self::RetireDeclaration(owner) => {
                drop(owner);
                Receipt::Retired
            }
            Self::Refresh(request) => Receipt::Refreshed(
                request
                    .prepare(&recite_compiler::authoring::CancellationToken::new())
                    .map(Box::new),
            ),
            Self::RetireRefresh(owner) => {
                drop(owner);
                Receipt::Retired
            }
        }
    }
}
