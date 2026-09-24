//! Scene and beat lookup uses the same loaded documents as the workbench.
use crate::editing::Writer;
use freya::prelude::WritableUtils;
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
    pub fn open(&self, mut writer: Writer) -> Result<(), String> {
        writer.remember_scene();
        let select = |model: &mut recite_writer_model::Workbench| {
            if let Some(beat) = &self.beat {
                model.inspect_block(beat)
            } else {
                model.show_script()
            }
        };
        let path = writer
            .files
            .peek()
            .as_ref()
            .and_then(|files| files.path_for_document(&self.document));
        if let Some(path) = path {
            writer
                .buffers
                .switch(writer.files, &path, writer.dark, select)?;
        } else if let Some(index) = WRITER_EXAMPLES
            .iter()
            .position(|example| example.name == self.document)
        {
            crate::examples::select_at(writer, index, select)?;
        } else {
            writer.try_navigate(select)?;
        }
        if let Some(beat) = &self.beat {
            writer.inspect(beat);
        } else {
            writer.ensure_beat()?;
        }
        if *writer.layout.view.peek() == recite_config::WriterView::Source {
            writer.layout.view.set(recite_config::WriterView::Script);
        }
        Ok(())
    }
}
pub(super) fn all(writer: Writer) -> Vec<Target> {
    let mut documents = if let Some(files) = writer.files.read().as_ref() {
        files.navigation_targets()
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
