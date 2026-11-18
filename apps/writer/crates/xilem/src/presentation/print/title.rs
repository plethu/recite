//! An action's words and, where they say less than the action does, its accessible name.
use xilem::masonry::core::ArcStr;

/// The words an action shows and the name assistive technology reads, such as "Continue" for
/// Continue preview in the preview's own head.
pub(crate) struct Title {
    pub(super) face: ArcStr,
    pub(super) name: Option<ArcStr>,
}
impl Title {
    /// `face` shown and `name` read aloud.
    pub(crate) fn named(face: impl Into<ArcStr>, name: impl Into<ArcStr>) -> Self {
        Self {
            face: face.into(),
            name: Some(name.into()),
        }
    }
}
impl From<&'static str> for Title {
    fn from(face: &'static str) -> Self {
        Self {
            face: face.into(),
            name: None,
        }
    }
}
impl From<String> for Title {
    fn from(face: String) -> Self {
        Self {
            face: face.into(),
            name: None,
        }
    }
}
impl From<ArcStr> for Title {
    fn from(face: ArcStr) -> Self {
        Self { face, name: None }
    }
}
