use crate::tests::support;

#[test]
fn status_phrases_are_separate_accessible_labels_without_separators() {
    let directory = support::project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n").unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.status = "Saved".into();
    let names = support::labels(&mut writer).unwrap();
    for phrase in ["Saved to disk", "no problems", "Saved"] {
        assert!(
            names.iter().any(|name| name == phrase),
            "{phrase}: {names:?}"
        );
    }
    assert!(
        !names
            .iter()
            .any(|name| name == "·" || name.starts_with("· ") || name.ends_with(" ·")),
        "{names:?}"
    );
}
