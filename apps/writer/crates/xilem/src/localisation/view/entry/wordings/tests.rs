use super::*;
use crate::localisation::tests::{fixture, key};
use crate::presentation::Theme;
use crate::tests::support;
use std::{fmt::Write as _, sync::Arc};
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{accesskit::Role, testing::TestHarness},
};

fn family(count: usize) -> (tempfile::TempDir, Writer) {
    let (directory, mut writer, mut driver) = fixture();
    let mut po =
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Default\"\n\n".to_owned();
    for index in 0..count {
        writeln!(po, "msgctxt \"11111111111111111111&wording_{index:06}\"\nmsgid \"Hello.\"\nmsgstr \"Version {index}\"\n").unwrap();
    }
    std::fs::write(directory.path().join("fr.po"), po).unwrap();
    writer.catalogue_action(work::Action::Reload);
    driver.finish(&mut writer);
    let key = EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id: PoEntryId::new(0),
    };
    writer.inspect_translation(&key);
    driver.finish(&mut writer);
    (directory, writer)
}

fn measure(count: usize) -> usize {
    let (_directory, mut writer) = family(count);
    let entry = writer.localisation.entry.as_ref().unwrap();
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = sized_box(flex_col((wordings(&writer, entry, false),)));
    let (pod, _) = view.build(&mut context, &mut writer);
    let harness = TestHarness::create_with_size(
        Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (900, 800),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let (mut nodes, mut wordings) = (0, 0);
    while let Some(node) = pending.pop() {
        nodes += 1;
        wordings += usize::from(
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Wording: ")),
        );
        pending.extend(node.children());
    }
    assert_eq!(wordings, 8);
    nodes
}

#[test]
fn wording_controls_stay_fixed_for_thousands_of_variants() {
    let small = measure(1_026);
    let large = measure(10_260);
    assert_eq!(small, large);
    assert!(large <= 64, "measured {large} wording-control nodes");
    println!("wording controls: {small}/{large} nodes; eight choices");
}

#[test]
fn wording_pages_preserve_the_current_translation_and_expose_later_entries() {
    let (_directory, mut writer) = family(24);
    let key = key(&writer);
    writer.update_translation(&key, 0, "Retained draft".into());
    support::click(&mut writer, &["More wordings"]).unwrap();
    let labels = support::labels(&mut writer).unwrap();
    assert!(
        labels
            .iter()
            .any(|label| label == "Wording: wording_000007")
    );
    assert_eq!(
        writer.localisation.slot.draft(key.id).unwrap().forms,
        ["Retained draft"]
    );
    support::click(&mut writer, &["Previous wordings"]).unwrap();
    assert_eq!(writer.localisation.entry.as_ref().unwrap().wording_page, 0);
}
