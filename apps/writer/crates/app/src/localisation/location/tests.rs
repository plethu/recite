// Fixtures here are shared with frontend tests through the `test-support` feature.
#![allow(unused_imports, dead_code, clippy::unwrap_used, clippy::expect_used)]
pub use super::super::tests::{fixture, form, key};
pub use super::*;
pub use crate::{navigation::Target as NavigationTarget, test_support as support};
pub use std::fs;
pub use tokio::sync::mpsc;

#[test]
pub fn dirty_catalogue_replacement_is_refused_before_document_selection_changes() {
    let (directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Keep this translation draft.".into());
    let before = writer.session.model().recovery();
    let candidate = directory.path().join("other.po");
    fs::copy(directory.path().join("fr.po"), &candidate).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.navigate(NavigationTarget::Location(Location {
        screen: LinkedScreen::Translations,
        catalogue: Some("other.po".into()),
        ..Location::default()
    }));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert_eq!(writer.session.model().recovery(), before);
    assert_eq!(form(&writer, &key), "Keep this translation draft.");
    assert_eq!(
        writer.localisation.catalogue_path(),
        Some(directory.path().join("fr.po").as_path())
    );
    assert!(
        writer.status.contains("Save or discard translation drafts"),
        "{}",
        writer.status
    );
}
