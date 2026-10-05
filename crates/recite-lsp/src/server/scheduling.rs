use super::{
    Publication, Server, ServerError,
    requests::RequestState,
    workers::{AnalysisJob, AnalysisResult, QueryJob, QueryResult},
};
use lsp_server::{ErrorCode, Notification, Response};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use recite_compiler::authoring::CancellationToken;
use std::sync::Arc;

impl Server {
    pub(super) fn prune_publications(&mut self) {
        self.output.retain(|publication| {
            let current = publication
                .fence
                .as_ref()
                .is_none_or(|fence| self.epochs.matches(fence));
            if !current
                && let lsp_server::Message::Notification(notification) = &publication.message
                && notification.method == PublishDiagnostics::METHOD
                && let Some(uri) = notification
                    .params
                    .get("uri")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|uri| uri.parse::<lsp_types::Uri>().ok())
            {
                defer_diagnostic(&mut self.dirty_diagnostics, uri);
            }
            current
        });
    }

    pub(super) fn known_scope(&self, uri: &lsp_types::Uri) -> Option<&str> {
        self.epochs
            .same_topology(&self.analyzed_epochs)
            .then(|| self.scopes.get(uri.as_str()))
            .flatten()
            .map(String::as_str)
    }
    pub(super) fn exclusive_update_partition(&self) -> Option<String> {
        if !self.epochs.same_topology(&self.analyzed_epochs) {
            return None;
        }
        let first = self.updates.front()?.1.changed_uri()?;
        let partition = self.scopes.get(first.as_str())?;
        self.updates
            .iter()
            .all(|(_, update)| {
                update
                    .changed_uri()
                    .and_then(|uri| self.scopes.get(uri.as_str()))
                    == Some(partition)
            })
            .then(|| partition.clone())
    }
    pub(super) fn schedule(&mut self) -> Result<(), ServerError> {
        if self.shutdown_requested {
            return Ok(());
        }
        if self.analyzing.is_none()
            && !self
                .output
                .iter()
                .any(|publication| publication.fence.is_some())
            && (self.analyzed_revision != Some(self.revision) || !self.dirty_diagnostics.is_empty())
        {
            self.analysis_partition = self.exclusive_update_partition();
            let control = CancellationToken::new();
            tracing::trace!(phase = "analysis_dispatch", revision = self.revision);
            self.workers
                .analysis
                .try_send(AnalysisJob {
                    through: self.revision,
                    epochs: self.epochs.clone(),
                    updates: self
                        .updates
                        .iter()
                        .map(|(_, update)| Arc::clone(update))
                        .collect(),
                    refresh_uris: self.dirty_diagnostics.clone(),
                    control: control.clone(),
                })
                .map_err(|_| ServerError::WorkerPanic)?;
            self.analyzing = Some(control);
        }
        if self.querying.is_none()
            && let Some(workspace) = &self.workspace
        {
            let next = self
                .requests
                .values_mut()
                .filter(|pending| {
                    matches!(pending.state, RequestState::Queued(_))
                        && self.analyzed_epochs.matches(&pending.fence)
                })
                .min_by_key(|pending| pending.serial);
            if let Some(pending) = next
                && let Some(query) = pending.dispatch()
            {
                tracing::trace!(phase = "query_dispatch", serial = pending.serial);
                self.workers
                    .query
                    .try_send(QueryJob {
                        serial: pending.serial,
                        query,
                        workspace: Arc::clone(workspace),
                        control: pending.control.clone(),
                    })
                    .map_err(|_| ServerError::WorkerPanic)?;
                self.querying = Some((pending.serial, pending.control.clone()));
            }
        }
        Ok(())
    }
    pub(super) fn analysis_finished(&mut self, result: AnalysisResult) -> Result<(), ServerError> {
        tracing::trace!(phase = "analysis_observed", revision = result.through);
        self.analyzing = None;
        self.analysis_partition = None;
        let Some(snapshot) = result.result.map_err(ServerError::Authoring)? else {
            return Ok(());
        };
        // A fully finished worker candidate may be retained even if a newer edit
        // arrived meanwhile. Publication and each query still have their own fence.
        self.updates
            .retain(|(revision, _)| *revision > result.through);
        self.analyzed_revision = Some(result.through);
        self.analyzed_epochs = result.epochs.clone();
        self.workspace = Some(snapshot.workspace);
        self.scopes = snapshot.scopes;
        if !self.shutdown_requested {
            for params in snapshot.diagnostics {
                let fence = result
                    .epochs
                    .fence(self.scopes.get(params.uri.as_str()).map(String::as_str));
                if self.epochs.matches(&fence) {
                    self.dirty_diagnostics.retain(|uri| uri != &params.uri);
                    self.enqueue(Publication {
                        message: Notification::new(PublishDiagnostics::METHOD.to_owned(), params)
                            .into(),
                        fence: Some(fence),
                    })?;
                } else {
                    defer_diagnostic(&mut self.dirty_diagnostics, params.uri);
                }
            }
        }
        Ok(())
    }
    pub(super) fn query_finished(&mut self, result: QueryResult) {
        tracing::trace!(phase = "query_observed", serial = result.serial);
        if self
            .querying
            .as_ref()
            .is_some_and(|(serial, _)| *serial == result.serial)
        {
            self.querying = None;
        }
        let Some((id, pending)) = self
            .requests
            .iter_mut()
            .find(|(_, pending)| pending.serial == result.serial)
        else {
            return;
        };
        if matches!(pending.state, RequestState::Running) {
            pending.state = RequestState::Ready(match result.result {
                Ok(value) => Response::new_ok(id.clone(), value),
                Err(error) => Response::new_err(id.clone(), ErrorCode::InternalError as i32, error),
            });
        }
    }
}

fn defer_diagnostic(uris: &mut Vec<lsp_types::Uri>, uri: lsp_types::Uri) {
    if !uris.contains(&uri) {
        uris.push(uri);
    }
}
