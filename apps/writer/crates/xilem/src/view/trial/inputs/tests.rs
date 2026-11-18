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
    let mut source = format!(":: start default\n> {bindings} @ffee6p8\n  Hello");
    for index in 0..count {
        write!(source, " {{name{index:03}}}").unwrap();
    }
    source.push_str(".\n-> END\n");
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.trial.settings.open = true;
    writer.trial.visible = true;
    writer.set_source_language_only(false);
    writer.set_preview_language_query("fr".into());
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = sized_box(editor(&writer, false, writer.script_width()));
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

/// Every text field the configured slip mounts: its name and its placeholder.
fn fields(writer: &mut Writer) -> Vec<(String, Option<String>)> {
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = sized_box(editor(writer, false, writer.script_width()));
    let (pod, _) = view.build(&mut context, writer);
    let harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        pod.new_widget,
        (900, 1000),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut fields = Vec::new();
    while let Some(node) = pending.pop() {
        if node.role() == masonry::accesskit::Role::TextInput {
            fields.push((
                node.label().unwrap_or_default(),
                node.placeholder().map(ToOwned::to_owned),
            ));
        }
        pending.extend(node.children());
    }
    fields
}

#[test]
fn source_only_hides_the_language_search_and_the_wording_placeholder_names_the_default() {
    let directory = project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.trial.settings.open = true;
    writer.trial.visible = true;
    let source_only = fields(&mut writer);
    assert!(
        source_only
            .iter()
            .any(|(name, placeholder)| name == "Preview wording"
                && placeholder.as_deref() == Some("Default")),
        "{source_only:?}"
    );
    assert!(
        !source_only
            .iter()
            .any(|(name, _)| name == "Find preview language")
    );
    writer.set_source_language_only(false);
    assert!(
        fields(&mut writer)
            .iter()
            .any(|(name, _)| name == "Find preview language")
    );
}
