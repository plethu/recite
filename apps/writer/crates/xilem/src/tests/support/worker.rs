//! Deliver real asynchronous worker messages through Xilem's public view boundary.
use super::*;
use std::sync::mpsc;
use xilem::core::{MessageResult, NoElement, View};

#[derive(Debug)]
struct Results(mpsc::Sender<(Arc<[ViewId]>, SendMessage)>);
impl RawProxy for Results {
    fn send_message(&self, path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.0
            .send((path, message))
            .map_err(|error| ProxyError::DriverFinished(error.0.1))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

pub(crate) struct Worker<S: 'static, V: View<S, (), ViewCtx, Element = NoElement>> {
    pub state: S,
    view: V,
    local: V::ViewState,
    // Drop queued receipts before the runtime can wait for their blocking workers.
    responses: mpsc::Receiver<(Arc<[ViewId]>, SendMessage)>,
    context: ViewCtx,
}
impl<S: 'static, V: View<S, (), ViewCtx, Element = NoElement>> Worker<S, V> {
    pub fn new(mut state: S, view: V) -> Self {
        let (sender, responses) = mpsc::channel();
        let mut context = ViewCtx::new(
            Arc::new(Results(sender)),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let (_, local) = view.build(&mut context, &mut state);
        Self {
            state,
            view,
            local,
            context,
            responses,
        }
    }
    pub fn next(&mut self) {
        let (path, output) = self
            .responses
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("worker terminal response");
        let mut message = MessageCtx::new(
            std::mem::take(self.context.environment()),
            path.to_vec(),
            output.into(),
        );
        let result = self
            .view
            .message(&mut self.local, &mut message, (), &mut self.state);
        assert!(
            !matches!(result, MessageResult::Stale),
            "live worker response must route to its view"
        );
        *self.context.environment() = message.finish().0;
    }
    pub fn close(mut self) -> S {
        self.view.teardown(&mut self.local, &mut self.context, ());
        self.state
    }
}
