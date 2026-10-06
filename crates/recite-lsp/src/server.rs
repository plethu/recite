use crate::workspace::LspWorkspace;
use crossbeam_channel::{never, select_biased};
use lsp_server::{Connection, Message, Notification};
use lsp_types::notification::{LogMessage, Notification as _};
use lsp_types::{InitializeParams, LogMessageParams, MessageType, Uri};
use recite_compiler::authoring::CancellationToken;
use recite_ui::UiCatalog;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

mod bootstrap;
mod error;
mod freshness;
mod notifications;
mod query;
mod requests;
mod scheduling;
mod text_sync;
mod trace;
mod updates;
mod workers;
#[allow(unused_imports, reason = "test harness reexports protocol seams")]
pub(crate) use bootstrap::run_connection_with_user_config;
#[allow(unused_imports, reason = "test harness reexports protocol seams")]
pub(crate) use bootstrap::{run_connection, run_connection_with_catalog};
pub use bootstrap::{run_stdio, run_stdio_with_catalog, run_stdio_with_locale};
pub use error::ServerError;
use freshness::{Epochs, Fence};
use requests::Requests;
use text_sync::Documents;
use updates::Update;
use workers::Workers;

struct Server {
    connection: Connection,
    workers: Workers,
    workspace: Option<Arc<LspWorkspace>>,
    scopes: BTreeMap<String, String>,
    epochs: Epochs,
    analyzed_epochs: Epochs,
    documents: Documents,
    updates: VecDeque<(u64, Arc<Update>)>,
    revision: u64,
    analyzed_revision: Option<u64>,
    analyzing: Option<CancellationToken>,
    analysis_partition: Option<String>,
    querying: Option<(u64, CancellationToken)>,
    requests: Requests,
    serial: u64,
    output: VecDeque<Publication>,
    dirty_diagnostics: Vec<Uri>,
    shutdown_requested: bool,
    exit_received: bool,
}
struct Publication {
    message: Message,
    fence: Option<Fence>,
}

impl Server {
    fn new(connection: Connection, params: InitializeParams, catalog: UiCatalog) -> Self {
        Self {
            connection,
            workers: Workers::start(params, catalog),
            workspace: None,
            scopes: BTreeMap::new(),
            epochs: Epochs::default(),
            analyzed_epochs: Epochs::default(),
            documents: Documents::default(),
            updates: VecDeque::new(),
            revision: 0,
            analyzed_revision: None,
            analyzing: None,
            analysis_partition: None,
            querying: None,
            requests: Requests::new(),
            serial: 0,
            output: VecDeque::new(),
            dirty_diagnostics: Vec::new(),
            shutdown_requested: false,
            exit_received: false,
        }
    }
    fn run(mut self) -> Result<(), ServerError> {
        let result = self.event_loop();
        if let Some(control) = self.analyzing {
            control.interrupt();
        }
        if let Some((_, control)) = self.querying {
            control.interrupt();
        }
        let joined = self.workers.join();
        result.and(joined)
    }
    fn event_loop(&mut self) -> Result<(), ServerError> {
        loop {
            // Bound ingress work so an active editor cannot indefinitely starve
            // worker completions or a ready writer. Publication follows the last
            // processed input, never a request's original dispatch decision.
            for _ in 0..32 {
                if !self.can_receive() {
                    break;
                }
                let Ok(message) = self.connection.receiver.try_recv() else {
                    break;
                };
                self.receive(message)?;
            }
            self.prune_publications();
            self.schedule()?;
            if self.exit_received && self.requests.is_empty() && self.output.is_empty() {
                return Ok(());
            }
            // Choose metadata before selection. The macro evaluates its send
            // expression only after that arm wins; until then cancellation and
            // freshness still belong to the authoritative pending state.
            let mut ready = self.requests.iter().find_map(|(id, pending)| {
                let stopped = matches!(pending.state, requests::RequestState::Stopped(_));
                if (self.output.is_empty() || stopped)
                    && matches!(
                        pending.state,
                        requests::RequestState::Ready(_) | requests::RequestState::Stopped(_)
                    )
                {
                    // Error construction is small, but belongs before a
                    // rendezvous writer can begin waiting for its packet.
                    let prepared = if stopped { pending.response(id) } else { None };
                    Some((id.clone(), prepared))
                } else {
                    None
                }
            });
            if let Some((id, _)) = &ready {
                tracing::trace!(phase = "output_ready", id = %id);
            } else if let Some(publication) = self.output.front() {
                trace::message("output_ready", &publication.message);
            }
            let (idle_sender, _idle_receiver) = crossbeam_channel::bounded(0);
            let sender = if ready.is_some() || !self.output.is_empty() {
                &self.connection.sender
            } else {
                &idle_sender
            };
            let input = if !self.can_receive() {
                never()
            } else {
                self.connection.receiver.clone()
            };
            let handed_off;
            select_biased! {
                send(sender, {
                    let message = if let Some((id, prepared)) = &mut ready {
                        prepared.take().or_else(|| self.requests.get_mut(id).and_then(|pending| pending.take_response(id)))
                            .unwrap_or_else(|| unreachable!("selected response remains ready until handoff"))
                            .into()
                    } else {
                        self.output.pop_front()
                            .unwrap_or_else(|| unreachable!("selected publication remains queued until handoff"))
                            .message
                    };
                    handed_off = tracing::enabled!(tracing::Level::TRACE)
                        .then(|| trace::Identity::of(&message));
                    message
                }) -> result => {
                    result.map_err(|_| ServerError::Send)?;
                    if let Some(identity) = handed_off { identity.record("handoff"); }
                    if let Some((id, _)) = ready {
                        self.requests.remove(&id);
                    }
                },
                recv(self.workers.analyzed) -> result => self.analysis_finished(result.map_err(|_| ServerError::WorkerPanic)?)?,
                recv(self.workers.queried) -> result => self.query_finished(result.map_err(|_| ServerError::WorkerPanic)?),
                recv(input) -> message => match message {
                    Ok(message) => self.receive(message)?,
                    Err(_) => return if self.shutdown_requested { Ok(()) } else { Err(ServerError::Disconnected) },
                },
            }
        }
    }
    fn receive(&mut self, message: Message) -> Result<(), ServerError> {
        trace::message("ingress", &message);
        match message {
            Message::Request(request) => self.request(request),
            Message::Notification(notification) => self.notification(notification),
            Message::Response(_) => Ok(()),
        }
    }
    fn publish_startup_warning(&mut self, message: String) -> Result<(), ServerError> {
        self.send(
            Notification::new(
                LogMessage::METHOD.to_owned(),
                LogMessageParams {
                    typ: MessageType::WARNING,
                    message,
                },
            )
            .into(),
        )
    }
    fn send(&mut self, message: Message) -> Result<(), ServerError> {
        self.enqueue(Publication {
            message,
            fence: None,
        })
    }
    fn can_receive(&self) -> bool {
        !self.exit_received && self.updates.len() < 256 && self.has_control_capacity()
    }
    fn has_control_capacity(&self) -> bool {
        // Pause ingress before accepting another message that may need an error
        // reply. A consuming client drains the queue without losing responses.
        self.output.len() < 256
            || self
                .output
                .iter()
                .filter(|item| item.fence.is_none())
                .take(256)
                .count()
                < 256
    }
    fn enqueue(&mut self, publication: Publication) -> Result<(), ServerError> {
        // Diagnostic output is one completed analysis batch, bounded by the
        // accepted workspace. Only miscellaneous protocol messages use this cap.
        if publication.fence.is_none() && !self.has_control_capacity() {
            return Err(ServerError::OutputCapacity);
        }
        self.output.push_back(publication);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
