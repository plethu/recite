//! Frontend worker receipts retire the complete navigation request off the UI thread.
use crate::project::{Checkpoint, ReadyCheckpoint};
pub(crate) use recite_writer_session::navigation::{
    Destination, Prepare, Prepared, PreparedDestination,
};

pub(crate) enum Work {
    Prepare(Box<Prepare>),
    Checkpoint(Box<Checkpoint>),
    RetireRequest(Box<super::request::Navigation>),
}
pub(crate) enum Output {
    Prepared(Result<Box<Prepared>, String>),
    Checkpoint(Result<ReadyCheckpoint, String>),
    Retired,
}
impl Work {
    pub(crate) fn execute(self) -> Output {
        match self {
            Self::Prepare(request) => Output::Prepared(
                request
                    .execute(&recite_compiler::authoring::CancellationToken::new())
                    .map(Box::new)
                    .map_err(|error| error.to_string()),
            ),
            Self::Checkpoint(request) => {
                Output::Checkpoint(request.wait().map_err(|e| e.to_string()))
            }
            Self::RetireRequest(request) => {
                drop(request);
                Output::Retired
            }
        }
    }
}
