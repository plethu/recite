//! Payload-free protocol boundaries for opt-in native timing experiments.

use lsp_server::Message;

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
