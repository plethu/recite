//! The component owns result publication and worker teardown.
use super::cache::SourceCompletionCache;
use crate::editing::Writer;
use freya::prelude::*;

pub(super) fn use_completion_work(writer: Writer) {
    let receiver = use_hook(move || {
        let service = SourceCompletionCache::new();
        let receiver = service.results();
        let mut cache = writer.source_viewport.cache;
        cache.set(Some(service));
        receiver
    });
    use_future(move || {
        let receiver = receiver.clone();
        let mut writer = writer;
        // Subscribe only to this stable receiver. Request/result State reads stay
        // inside the future so they cannot restart the result task.
        async move {
            while let Ok(output) = receiver.recv().await {
                let mut error = None;
                let mut cache = Writable::from(writer.source_viewport.cache);
                cache.write_if(|mut cache| {
                    let Some(service) = cache
                        .as_mut()
                        .filter(|service| service.matches_receiver(&receiver))
                    else {
                        return false;
                    };
                    let (changed, failure) = service.accept(output);
                    error = failure;
                    changed
                });
                if let Some(error) = error {
                    writer.message.error(error);
                }
            }
        }
    });
    use_drop(move || {
        let mut cache = writer.source_viewport.cache;
        cache.set(None);
    });
}
