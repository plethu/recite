//! Watch the containing directory so atomic PO replacement stays observable.
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
            let paths = watch
                .peek()
                .as_ref()
                .and_then(|watch| watch.changed().ok())
                .unwrap_or_default();
            let changed = writer
                .localisation
                .peek()
                .catalogue
                .as_ref()
                .is_some_and(|c| {
                    paths.contains(&c.path)
                        && super::Catalogue::open(&c.path).map_or(true, |disk| {
                            disk.document.fingerprint() != c.document.fingerprint()
                        })
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
        rect()
    }
}
