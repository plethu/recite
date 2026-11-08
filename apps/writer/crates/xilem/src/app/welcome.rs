//! Project access keeps form fields mounted and submission controls in view.
use super::App;
use xilem::masonry::kurbo::Size;
use xilem::masonry::{
    core::{Property, UsesProperty},
    widgets::TextInput,
};
mod feedback;
mod view;
pub(in crate::app) use view::panel;

/// Actual native extents let short forms stay together and tall forms scroll.
#[derive(Default)]
pub(super) struct Measurements {
    content: Option<Size>,
    actions: Option<Size>,
    available: Option<Size>,
}
impl Measurements {
    fn flex(&self) -> (f64, f64) {
        let (Some(content), Some(actions), Some(available)) =
            (self.content, self.actions, self.available)
        else {
            return (1., 0.);
        };
        let remaining = (available.height - actions.height).max(0.);
        if content.height <= 0. || remaining <= 0. {
            return (1., 0.);
        }
        (
            content.height.min(remaining),
            (remaining - content.height).max(0.),
        )
    }
}

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ProjectField(pub bool);
impl UsesProperty<ProjectField> for TextInput {}
impl Property for ProjectField {
    fn static_default() -> &'static Self {
        static NONE: ProjectField = ProjectField(false);
        &NONE
    }
}
