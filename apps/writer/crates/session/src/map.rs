//! Shared map geometry and personal placement; dialogue semantics stay in the model.
pub mod camera;
pub mod layout;
pub mod navigation;
pub mod placement;
pub mod routing;
pub mod scope;
pub mod synopsis;
pub mod topology;

/// Prose is replaced by a structural overview below this camera scale.
pub const DIALOGUE_ZOOM: f32 = 0.6;
