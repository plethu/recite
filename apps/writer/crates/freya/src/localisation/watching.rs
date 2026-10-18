//! Watch the containing directory so atomic PO replacement stays observable.
mod worker;
use crate::{
    editing::Writer,
    external::Watch,
    messages::{MsgId, text},
};
use freya::prelude::*;
#[derive(Clone, Copy)]
pub(crate) struct CatalogueWatch {
    pub writer: Writer,
}
impl PartialEq for CatalogueWatch {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for CatalogueWatch {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut watch = use_state(|| None::<Watch>);
        let mut worker = use_state(worker::Worker::new);
        let results = use_hook(move || worker.peek().results());
        use_future(move || {
            let results = results.clone();
            async move {
                while let Ok(output) = results.recv().await {
                    accept(writer, output);
                }
            }
        });
        let path = use_memo(move || {
            writer
                .localisation
                .read()
                .catalogue
                .as_ref()
                .map(|c| c.path.clone())
        });
        use_side_effect(move || {
            watch.set(path.read().as_ref().and_then(|p| Watch::parent(p).ok()));
        });
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(500));
        if tick.elapsed() {
            tick.reset();
            if let Some(error) = writer
                .localisation
                .peek()
                .catalogue
                .as_ref()
                .and_then(|c| c.recovery_error())
            {
                writer.message.error(format!(
                    "Translation recovery failed: {error}. Save the catalogue to keep your work."
                ));
            }
            if !writer
                .localisation
                .peek()
                .catalogue
                .as_ref()
                .is_some_and(|c| c.saving)
            {
                let paths = watch
                    .peek()
                    .as_ref()
                    .and_then(|watch| watch.changed().ok())
                    .unwrap_or_default();
                if let Some(catalogue) = writer.localisation.peek().catalogue.as_ref()
                    && paths.contains(&catalogue.path)
                    && let Err(error) = worker.write().submit(worker::Request {
                        path: catalogue.path.clone(),
                        baseline: catalogue.saved_fingerprint().clone(),
                    })
                {
                    writer.message.error(error.to_string());
                }
            }
        }
        rect()
    }
}

fn accept(mut writer: Writer, output: worker::Output) {
    let changed = writer
        .localisation
        .peek()
        .catalogue
        .as_ref()
        .is_some_and(|c| {
            !c.saving
                && c.path == output.request.path
                && c.saved_fingerprint() == &output.request.baseline
                && output
                    .fingerprint
                    .as_ref()
                    .map_or(true, |disk| disk != c.saved_fingerprint())
        });
    if changed && !writer.message.is_error() {
        writer.message.error_with_action(
            text(MsgId::WriterCatalogueChanged),
            text(MsgId::WriterCompare),
            EventHandler::new(move |()| {
                writer.pane.set(crate::editing::Pane::Map);
                writer.localisation.write().active = true;
                super::comparison::prepare(writer);
            }),
        );
    }
}
