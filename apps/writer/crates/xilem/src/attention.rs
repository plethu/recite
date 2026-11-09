//! Frontend views and native glue over `recite_writer_app::attention`.
pub(crate) use recite_writer_app::attention::*;
#[cfg(test)]
mod tests;
use xilem::masonry::core::{Widget, WidgetId, WidgetRef};
use xilem::masonry::{core, widgets};

/// The passage an inline text field edits, read back from the native focus target.
#[derive(Clone, Default, PartialEq)]
pub struct PassageField(pub Option<EditingTarget>);
impl PassageField {
    pub fn focused(
        mut root: WidgetRef<'_, dyn Widget>,
        focused: WidgetId,
    ) -> Option<EditingTarget> {
        loop {
            if let Some(input) = root.downcast::<widgets::TextInput>() {
                return (input.area_pod().id() == focused)
                    .then(|| input.get_prop::<Self>().0.clone())
                    .flatten();
            }
            root = root
                .children()
                .into_iter()
                .find(|child| child.ctx().has_focus_target())?;
        }
    }
}
impl core::UsesProperty<PassageField> for widgets::TextInput {}
impl core::Property for PassageField {
    fn static_default() -> &'static Self {
        static OTHER: PassageField = PassageField(None);
        &OTHER
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Region(pub Option<Pane>);
/// A contextual pane's preferred entry, without changing its geometry or Tab order.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct PaneEntry(pub bool);
/// Explicit pane entry reveals this small field group, then prioritises its editable control.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct EntryContext(pub bool);
impl core::UsesProperty<EntryContext> for widgets::Flex {}
impl core::Property for EntryContext {
    fn static_default() -> &'static Self {
        static NONE: EntryContext = EntryContext(false);
        &NONE
    }
}
impl core::UsesProperty<PaneEntry> for widgets::Flex {}
impl core::Property for PaneEntry {
    fn static_default() -> &'static Self {
        static NONE: PaneEntry = PaneEntry(false);
        &NONE
    }
}
impl core::UsesProperty<Region> for widgets::SizedBox {}
impl core::UsesProperty<Region> for widgets::Flex {}
impl core::Property for Region {
    fn static_default() -> &'static Self {
        static NONE: Region = Region(None);
        &NONE
    }
}
