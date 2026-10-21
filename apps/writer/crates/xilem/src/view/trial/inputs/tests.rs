use super::*;
use crate::tests::support::{self, click, labels, open, project};
use masonry::testing::TestHarness;
use std::{fmt::Write as _, sync::Arc};
use xilem::{ViewCtx, core::View as _};

fn binding_view(count: usize) -> (usize, Writer) {
    let bindings = (0..count)
        .map(|index| format!("bind=(name{index:03}:string=$person{index:03})"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut source = format!(":: start default\n> line@11111111111111111111 {bindings}\n  Hello");
    for index in 0..count {
        write!(source, " {{name{index:03}}}").unwrap();
    }
    source.push_str(".\n-> END\n");
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.trial.settings.open = true;
    writer.trial.settings.query = "fr".into();
    writer.search_preview_languages();
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = sized_box(controls(&writer, false));
    let (pod, _) = view.build(&mut context, &mut writer);
    let harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        pod.new_widget,
        (900, 1000),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut fields = 0;
    let mut nodes = 0;
    while let Some(node) = pending.pop() {
        nodes += 1;
        if node.role() == masonry::accesskit::Role::TextInput
            && node
                .label()
                .is_some_and(|label| label.starts_with("Preview value:"))
        {
            fields += 1;
        }
        pending.extend(node.children());
    }
    assert_eq!(fields, 4);
    (nodes, writer)
}

#[test]
fn interpolation_control_cost_is_fixed_and_paging_keeps_values() {
    let (small, _) = binding_view(32);
    let (large, mut writer) = binding_view(320);
    assert_eq!(
        small, large,
        "control count must not grow with total bindings"
    );
    assert!(
        large <= 132,
        "configured preview controls: {small}/{large} nodes"
    );
    writer
        .trial
        .settings
        .values
        .insert("person000".into(), "Retained".into());
    click(&mut writer, &["More preview values"]).unwrap();
    let mounted = labels(&mut writer).unwrap();
    assert!(
        mounted
            .iter()
            .any(|label| label == "Preview value: person004")
    );
    assert!(
        !mounted
            .iter()
            .any(|label| label == "Preview value: person000")
    );
    click(&mut writer, &["Previous preview values"]).unwrap();
    assert_eq!(writer.trial.settings.values["person000"], "Retained");
}
