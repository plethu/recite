//! One small placement intent; copying the saved layout belongs to preparation.
use super::*;
#[derive(Clone, Default)]
pub(super) enum Status {
    #[default]
    Ready,
    LoadFailed(String),
    SaveFailed(String),
}
impl Status {
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Ready => None,
            Self::LoadFailed(message) | Self::SaveFailed(message) => Some(message),
        }
    }
    pub fn needs_save(&self) -> bool {
        matches!(self, Self::SaveFailed(_))
    }
}
#[derive(Clone, PartialEq)]
pub(super) enum Change {
    Move {
        beat: String,
        card: String,
        node: Node,
    },
    Reset,
}
impl Change {
    pub fn apply(&self, positions: &mut Positions) {
        match self {
            Self::Move { card, node, .. } => {
                positions.insert(card.clone(), (node.x, node.y));
            }
            Self::Reset => positions.clear(),
        }
    }
}
