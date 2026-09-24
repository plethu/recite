//! Resolve catalogue context through the current scene or extracted PO metadata.
use crate::editing::Writer;
use recite_core::PoEntry;
use recite_writer_model::Passage;

#[derive(Clone)]
pub(super) struct Destination {
    document: Option<String>,
    beat: String,
    anchor: String,
}
impl Destination {
    pub fn resolve(entry: &PoEntry, passages: &[Passage]) -> Option<Self> {
        let anchor = entry.context()?.split('&').next()?;
        if let Some(passage) = passages
            .iter()
            .find(|p| p.id == anchor && p.text == entry.source_text())
        {
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
            || crate::palette::display_name(&self.beat),
            |document| format!("{document} · {}", crate::palette::display_name(&self.beat)),
        )
    }
    pub fn open(&self, writer: Writer) -> bool {
        crate::navigation::open_passage(writer, self.document.as_deref(), &self.beat, &self.anchor)
    }
}
