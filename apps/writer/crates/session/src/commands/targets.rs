//! Scene and beat lookup uses accepted sources; fields awaiting Apply are excluded.
use crate::location::{Location, Screen};
use recite_writer_model::{WRITER_EXAMPLES, Workbench, WorkbenchError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub document: String,
    pub beat: Option<String>,
}
impl Target {
    pub fn label(&self) -> String {
        self.beat.as_ref().map_or_else(
            || self.document.clone(),
            |beat| format!("{} · {}", self.document, crate::choices::display_name(beat)),
        )
    }
    pub fn location(&self, origin: Location) -> Location {
        Location {
            screen: Screen::Write,
            passage: None,
            source: false,
            document: self.document.clone(),
            beat: self.beat.clone(),
            view: Some(recite_config::WriterView::Script),
            ..origin
        }
    }
}
pub fn project(current: &Workbench) -> Vec<Target> {
    with_current(current, current.document().project_sections())
}
pub fn examples(current: &Workbench) -> Result<Vec<Target>, WorkbenchError> {
    let documents = WRITER_EXAMPLES
        .iter()
        .map(|example| {
            let model = example.open()?;
            Ok((
                model.document().key().to_string(),
                model.document().sections(),
            ))
        })
        .collect::<Result<_, WorkbenchError>>()?;
    Ok(with_current(current, documents))
}
fn with_current(current: &Workbench, mut documents: Vec<(String, Vec<String>)>) -> Vec<Target> {
    let name = current.document().key().to_string();
    let sections = current.document().sections();
    if let Some((_, beats)) = documents.iter_mut().find(|(key, _)| key == &name) {
        *beats = sections;
    } else {
        documents.insert(0, (name, sections));
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
pub fn rank(label: &str, query: &str) -> Option<u8> {
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

/// Stable ranking for an explicit lookup; retained cursors resolve again against current targets.
pub fn matching(targets: impl IntoIterator<Item = Target>, query: &str) -> Vec<Target> {
    let mut matches = targets
        .into_iter()
        .filter_map(|target| rank(&target.label(), query).map(|rank| (rank, target)))
        .collect::<Vec<_>>();
    matches.sort_by_key(|(rank, _)| *rank);
    matches.into_iter().map(|(_, target)| target).collect()
}

#[cfg(test)]
mod tests;
