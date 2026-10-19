//! Scene and beat lookup uses the same loaded documents as the workbench.
use crate::editing::Writer;
use recite_writer_model::WRITER_EXAMPLES;

#[derive(Clone, PartialEq)]
pub(super) struct Target {
    pub document: String,
    pub beat: Option<String>,
}
impl Target {
    pub fn label(&self) -> String {
        self.beat.as_ref().map_or_else(
            || self.document.clone(),
            |beat| format!("{} · {}", self.document, crate::palette::display_name(beat)),
        )
    }
    pub fn open(&self, writer: Writer) -> Result<(), String> {
        let location = crate::navigation::location::Location {
            screen: crate::navigation::location::Screen::Write,
            passage: None,
            source: false,
            document: self.document.clone(),
            beat: self.beat.clone(),
            view: Some(recite_config::WriterView::Script),
            ..crate::navigation::snapshot(writer)
        };
        crate::navigation::route(
            writer,
            location,
            None,
            freya::prelude::EventHandler::new(move |result: Result<(), String>| {
                let mut message = writer.message;
                if let Err(error) = result {
                    message.error(error);
                } else if !writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .is_ok_and(crate::scene::has_no_beats)
                    && let Err(error) = writer.ensure_beat()
                {
                    message.error(error);
                }
            }),
        )
    }
}
pub(super) fn all(writer: Writer) -> Vec<Target> {
    let mut documents = if writer.files.read().is_some() {
        writer
            .buffers
            .model
            .read()
            .as_ref()
            .map(|model| model.document().project_sections())
            .unwrap_or_default()
    } else {
        WRITER_EXAMPLES
            .iter()
            .filter_map(|e| e.open().ok())
            .map(|m| (m.document().key().to_string(), m.document().sections()))
            .collect()
    };
    if let Ok(model) = writer.buffers.model.read().as_ref() {
        let name = model.document().key().to_string();
        let sections = model.document().sections();
        if let Some((_, beats)) = documents.iter_mut().find(|(key, _)| key == &name) {
            *beats = sections;
        } else {
            documents.insert(0, (name, sections));
        }
    }
    documents
        .into_iter()
        .flat_map(|(document, beats)| {
            std::iter::once(Target {
                document: document.clone(),
                beat: None,
            })
            .chain(beats.into_iter().map(move |beat| Target {
                document: document.clone(),
                beat: Some(beat),
            }))
        })
        .collect()
}
pub(super) fn rank(label: &str, query: &str) -> Option<u8> {
    let label = label.to_lowercase();
    let query = query.trim().to_lowercase();
    if label == query {
        Some(0)
    } else if label.starts_with(&query) {
        Some(1)
    } else if query.split_whitespace().all(|word| label.contains(word)) {
        Some(2)
    } else {
        None
    }
}
