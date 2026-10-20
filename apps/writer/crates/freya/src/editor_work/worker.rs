//! Bounded preparation and durability work; accepted writes are never cancelled.
use crate::localisation::catalogue::{CatalogueDurability, CatalogueSave, CatalogueSaved};
use crate::navigation::worker;
use crate::project::FileError;
use recite_writer_model::{EditRequest, PreparedEdit, ReplyRules, ReplyRulesRequest};
use recite_writer_session::project::save::sequence;
pub(crate) enum Work {
    RetirePresentation(Box<crate::presentation::RetiredPresentation>),
    Navigation(Box<worker::Work>),
    Edit(EditRequest),
    RetireEdit(Box<PreparedEdit>),
    Rules(ReplyRulesRequest),
    RetireRules(Box<(ReplyRulesRequest, Option<Box<ReplyRules>>)>),
    Catalogue(Box<CatalogueSave>),
    CatalogueRecovery(CatalogueDurability),
    RetireCatalogue(Box<CatalogueSave>),
    Save(sequence::Work),
}
pub(crate) enum Output {
    Navigation(worker::Output),
    Edit(Result<Box<PreparedEdit>, recite_writer_model::WorkbenchError>),
    Rules(ReplyRulesRequest, Result<Box<ReplyRules>, String>),
    Catalogue(Box<CatalogueSave>, Result<CatalogueSaved, String>),
    Retired,
    Recovered(Result<(), FileError>),
    Save(sequence::Receipt),
}
pub(crate) struct Worker {
    requests: async_channel::Sender<Work>,
    results: async_channel::Receiver<Output>,
}
impl Worker {
    pub(super) fn new() -> Result<Self, FileError> {
        let (requests, waiting) = async_channel::bounded::<Work>(1);
        let (output, results) = async_channel::bounded(1);
        let retired_results = results.clone();
        std::thread::Builder::new()
            .name("writer-work".into())
            .spawn(move || {
                while let Ok(work) = waiting.recv_blocking() {
                    let result = match work {
                        Work::RetirePresentation(retired) => {
                            drop(retired);
                            Output::Retired
                        }
                        Work::Navigation(work) => Output::Navigation(work.execute()),
                        Work::Edit(request) => Output::Edit(
                            request
                                .prepare(&recite_compiler::authoring::CancellationToken::new())
                                .map(Box::new),
                        ),
                        Work::RetireEdit(value) => {
                            drop(value);
                            Output::Retired
                        }
                        Work::Rules(request) => {
                            let result = request.prepare().map(Box::new).map_err(|e| e.to_string());
                            Output::Rules(request, result)
                        }
                        Work::RetireRules(value) => {
                            drop(value);
                            Output::Retired
                        }
                        Work::Catalogue(request) => {
                            let result = request.execute();
                            Output::Catalogue(request, result)
                        }
                        Work::CatalogueRecovery(ticket) => Output::Recovered(ticket.wait()),
                        Work::RetireCatalogue(value) => {
                            drop(value);
                            Output::Retired
                        }
                        Work::Save(work) => Output::Save(work.execute()),
                    };
                    if output.send_blocking(result).is_err() {
                        break;
                    }
                }
                // Keep the last output owner here if root teardown closes delivery.
                while let Ok(result) = retired_results.try_recv() {
                    drop(result);
                }
                while let Ok(work) = waiting.try_recv() {
                    drop(work);
                }
            })?;
        Ok(Self { requests, results })
    }
    pub(crate) fn retire_presentation(
        retired: crate::presentation::RetiredPresentation,
    ) -> Result<(), FileError> {
        let worker = Self::new()?;
        worker
            .requests
            .try_send(Work::RetirePresentation(Box::new(retired)))
            .map_err(|_| stopped())?;
        // Closing delivery leaves the queued owner with the existing worker's drain loop.
        Ok(())
    }
    pub(crate) async fn execute(&self, work: Work) -> Result<Output, FileError> {
        self.requests.send(work).await.map_err(|_| stopped())?;
        self.results.recv().await.map_err(|_| stopped())
    }
}
fn stopped() -> FileError {
    std::io::Error::other("Editor worker stopped unexpectedly. Keep the editor open and retry.")
        .into()
}
impl Drop for Worker {
    fn drop(&mut self) {
        // An active filesystem operation finishes. Closing delivery never rolls it
        // back or pretends it failed to happen; the existing recovery remains.
        self.requests.close();
        self.results.close();
    }
}

#[cfg(test)]
mod tests;
