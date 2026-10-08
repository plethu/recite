//! Payload-free protocol boundaries for opt-in native timing experiments.

use lsp_server::{Message, RequestId};

pub(super) enum Identity {
    Request(RequestId, String),
    Response(RequestId),
    Notification(String),
}

impl Identity {
    pub(super) fn of(message: &Message) -> Self {
        match message {
            Message::Request(request) => Self::Request(request.id.clone(), request.method.clone()),
            Message::Response(response) => Self::Response(response.id.clone()),
            Message::Notification(notification) => Self::Notification(notification.method.clone()),
        }
    }

    pub(super) fn record(&self, phase: &'static str) {
        match self {
            Self::Request(id, method) => tracing::trace!(phase, id = %id, method = %method),
            Self::Response(id) => tracing::trace!(phase, id = %id),
            Self::Notification(method) => tracing::trace!(phase, method = %method),
        }
    }
}

pub(super) fn message(phase: &'static str, message: &Message) {
    match message {
        Message::Request(request) => {
            tracing::trace!(phase, id = %request.id, method = %request.method);
        }
        Message::Response(response) => {
            tracing::trace!(phase, id = %response.id);
        }
        Message::Notification(notification) => {
            tracing::trace!(phase, method = %notification.method);
        }
    }
}
