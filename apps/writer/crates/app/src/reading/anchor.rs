//! Where reading was anchored, so a frontend keeps its scroll position across window shifts.
use super::ReadingWindow;
use recite_core::DocumentKey;

#[derive(Clone, PartialEq)]
struct AnchorLocation {
    document: DocumentKey,
    beat: Option<String>,
    window: ReadingWindow,
}
/// Where reading was anchored, so a frontend can keep its scroll position across window shifts
/// and reset it for explicit jumps.
#[derive(Clone, Default, PartialEq)]
pub struct Anchor(Option<AnchorLocation>);
impl Anchor {
    /// No manuscript is anchored.
    pub const NONE: Self = Self(None);
    pub fn capture(writer: &crate::Writer) -> Self {
        Self(Some(AnchorLocation {
            document: writer.session.model().document().key().clone(),
            beat: writer.reading.selected.clone(),
            window: writer.reading.window,
        }))
    }
    /// The same beat moved its reading window rather than jumping elsewhere.
    pub fn shifted_from(&self, previous: &Self) -> bool {
        match (&self.0, &previous.0) {
            (Some(current), Some(previous)) => {
                current.document == previous.document
                    && current.beat == previous.beat
                    && current.window.is_shift()
                    && current.window.start() != previous.window.start()
            }
            _ => false,
        }
    }
    pub fn is_manuscript(&self) -> bool {
        self.0.is_some()
    }
}
