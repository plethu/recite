//! The popup owns one request; only its matching result can be selected.
use super::{
    request::{Completed, Request},
    worker::{Output, Worker},
};
use crate::editing::Writer;
use freya::prelude::*;
use std::sync::Arc;

enum ResultState {
    Pending,
    Ready(Arc<Completed>),
    Failed,
}
struct Current {
    request: Arc<Request>,
    result: ResultState,
}
pub(crate) struct SourceCompletionCache {
    worker: Worker,
    current: Option<Current>,
}
impl SourceCompletionCache {
    pub(super) fn new() -> Self {
        Self {
            worker: Worker::new(),
            current: None,
        }
    }
    pub(super) fn results(&self) -> async_channel::Receiver<Output> {
        self.worker.results()
    }
    pub(super) fn matches_receiver(&self, receiver: &async_channel::Receiver<Output>) -> bool {
        self.worker.results().same_channel(receiver)
    }
    pub(super) fn pending(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|current| matches!(current.result, ResultState::Pending))
    }
    pub(super) fn clear(&mut self) -> bool {
        self.worker.cancel();
        self.current.take().is_some()
    }
    fn request(&mut self, request: Request) -> Result<(), String> {
        let request = Arc::new(request);
        self.current = Some(Current {
            request: Arc::clone(&request),
            result: ResultState::Pending,
        });
        if let Err(error) = self.worker.submit(request) {
            if let Some(current) = &mut self.current {
                current.result = ResultState::Failed;
            }
            return Err(format!("Could not start source suggestions: {error}"));
        }
        Ok(())
    }
    pub(super) fn accept(&mut self, output: Output) -> (bool, Option<String>) {
        let Some(current) = &mut self.current else {
            return (false, None);
        };
        match output {
            Output::Completed(completed) if Arc::ptr_eq(&current.request, &completed.request) => {
                current.result = ResultState::Ready(completed);
                (true, None)
            }
            Output::Failed { request, message }
                if request
                    .as_ref()
                    .is_none_or(|request| Arc::ptr_eq(request, &current.request)) =>
            {
                current.result = ResultState::Failed;
                (true, Some(message))
            }
            _ => (false, None),
        }
    }
    pub(super) fn owns(&self, completed: &Completed) -> bool {
        self.current.as_ref().is_some_and(|current| {
            Arc::ptr_eq(&current.request,&completed.request)
                && matches!(&current.result,ResultState::Ready(ready) if std::ptr::eq(ready.as_ref(),completed))
        })
    }
}
pub(super) fn clear(mut writer: Writer) {
    writer.source_viewport.candidate.set_if_modified(None);
    let mut cache = Writable::from(writer.source_viewport.cache);
    cache.write_if(|mut cache| cache.as_mut().is_some_and(SourceCompletionCache::clear));
}
pub(super) fn candidates(mut writer: Writer) -> Option<Arc<Completed>> {
    let request = Request::capture(writer)?;
    let cache = writer.source_viewport.cache;
    {
        let cache = cache.read();
        let service = cache.as_ref()?;
        if let Some(current) = &service.current
            && current.request.matches(&request)
        {
            return match &current.result {
                ResultState::Ready(items) => Some(Arc::clone(items)),
                _ => None,
            };
        }
    }
    writer.source_viewport.candidate.set_if_modified(None);
    let outcome = writer
        .source_viewport
        .cache
        .write()
        .as_mut()?
        .request(request);
    if let Err(error) = outcome {
        writer.message.error(error);
    }
    None
}

#[cfg(test)]
mod tests;
