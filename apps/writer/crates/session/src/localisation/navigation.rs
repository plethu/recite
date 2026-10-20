//! Resolve a translation to its source location without native routing.
use recite_core::po::PoEntry;
use recite_writer_model::Passage;
#[derive(Clone)]
pub struct Destination {
    pub document: Option<String>,
    pub beat: String,
    pub anchor: String,
}
impl Destination {
    pub fn resolve(entry: &PoEntry, passages: &[Passage]) -> Option<Self> {
        let anchor = entry.context()?.split('&').next()?;
        let passage = passages
            .iter()
            .find(|p| p.id == anchor && p.text == entry.source_text());
        Self::with_current_passage(entry, passage)
    }
    pub fn with_current_passage(entry: &PoEntry, passage: Option<&Passage>) -> Option<Self> {
        let anchor = entry.context()?.split('&').next()?;
        if let Some(passage) = passage {
            return Some(Self {
                document: None,
                beat: passage.section.clone(),
                anchor: anchor.into(),
            });
        }
        let metadata = |prefix: &str| {
            entry.comments().iter().find_map(|c| {
                c.text()
                    .strip_prefix(prefix)
                    .map(str::trim)
                    .map(str::to_owned)
            })
        };
        Some(Self {
            document: Some(metadata("file:")?),
            beat: metadata("block:")?,
            anchor: anchor.into(),
        })
    }
    pub fn caption(&self) -> String {
        self.document.as_ref().map_or_else(
            || crate::choices::display_name(&self.beat),
            |document| format!("{document} · {}", crate::choices::display_name(&self.beat)),
        )
    }
}
