use super::{Server, ServerError, freshness::Fence, query::Query};
use lsp_server::{ErrorCode, Request, RequestId, Response};
use recite_compiler::authoring::CancellationToken;
use std::collections::BTreeMap;

pub(super) struct Pending {
    pub(super) serial: u64,
    pub(super) fence: Fence,
    pub(super) control: CancellationToken,
    pub(super) state: RequestState,
}
pub(super) enum RequestState {
    Queued(Query),
    Running,
    Ready(Response),
    Stopped(StopReason),
}
#[derive(Clone, Copy)]
pub(super) enum StopReason {
    Cancelled,
    Stale,
    Shutdown,
}
pub(super) type Requests = BTreeMap<RequestId, Pending>;

impl Server {
    pub(super) fn request(&mut self, request: Request) -> Result<(), ServerError> {
        if self.requests.contains_key(&request.id) {
            return Err(ServerError::DuplicateRequest);
        }
        if self.shutdown_requested {
            return self.send(
                Response::new_err(
                    request.id,
                    ErrorCode::InvalidRequest as i32,
                    "server is shutting down".to_owned(),
                )
                .into(),
            );
        }
        if request.method == "shutdown" {
            self.shutdown_requested = true;
            for pending in self.requests.values_mut() {
                pending.stop(StopReason::Shutdown);
            }
            if let Some(control) = &self.analyzing {
                control.interrupt();
            }
            self.updates.clear();
            return self.send(Response::new_ok(request.id, ()).into());
        }
        if self.requests.len() >= 64 {
            return self
                .send(failure(request.id, "server_busy", "request capacity exhausted").into());
        }
        let id = request.id.clone();
        let query = match Query::parse(request) {
            Ok(query) => query,
            Err(response) => return self.send((*response).into()),
        };
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or(ServerError::SequenceExhausted)?;
        let fence = self.epochs.fence(self.known_scope(query.uri()));
        tracing::trace!(phase = "queued", id = %id, serial = self.serial);
        self.requests.insert(
            id,
            Pending {
                serial: self.serial,
                fence,
                control: CancellationToken::new(),
                state: RequestState::Queued(query),
            },
        );
        Ok(())
    }
    pub(super) fn cancel(&mut self, params: serde_json::Value) {
        if let Some(id) = params
            .get("id")
            .and_then(|id| serde_json::from_value::<RequestId>(id.clone()).ok())
            && let Some(pending) = self.requests.get_mut(&id)
        {
            pending.stop(StopReason::Cancelled);
        }
    }
    pub(super) fn invalidate(&mut self) {
        for pending in self.requests.values_mut() {
            if !self.epochs.matches(&pending.fence) {
                pending.stop(StopReason::Stale);
            }
        }
    }
}
impl Pending {
    pub(super) fn dispatch(&mut self) -> Option<Query> {
        match std::mem::replace(&mut self.state, RequestState::Running) {
            RequestState::Queued(query) => Some(query),
            state => {
                self.state = state;
                None
            }
        }
    }
    pub(super) fn stop(&mut self, reason: StopReason) {
        // Client cancellation wins until handoff, including a previously stale candidate.
        if !matches!(self.state, RequestState::Stopped(StopReason::Cancelled)) {
            self.state = RequestState::Stopped(reason);
        }
        self.control.interrupt();
    }
    pub(super) fn take_response(&mut self, id: &RequestId) -> Option<Response> {
        match std::mem::replace(&mut self.state, RequestState::Running) {
            RequestState::Ready(response) => Some(response),
            state => {
                self.state = state;
                self.response(id)
            }
        }
    }
    pub(super) fn response(&self, id: &RequestId) -> Option<Response> {
        match &self.state {
            RequestState::Queued(_) | RequestState::Running => None,
            RequestState::Ready(response) => Some(response.clone()),
            RequestState::Stopped(reason) => Some(match reason {
                StopReason::Cancelled => {
                    Response::new_err(id.clone(), -32800, "request cancelled".to_owned())
                }
                StopReason::Stale => failure(
                    id.clone(),
                    "stale_snapshot",
                    "document or project changed during request",
                ),
                StopReason::Shutdown => failure(id.clone(), "shutdown", "server is shutting down"),
            }),
        }
    }
}
pub(super) fn failure(id: RequestId, reason: &str, message: &str) -> Response {
    let mut response = Response::new_err(id, -32803, message.to_owned());
    if let Some(error) = &mut response.error {
        error.data = Some(serde_json::json!({"reason": reason}));
    }
    response
}
