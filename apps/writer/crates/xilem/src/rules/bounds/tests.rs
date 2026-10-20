use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::accesskit;
use xilem::{
    ViewCtx,
    core::View,
    masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
};

#[test]
fn a_thousand_effects_keep_only_one_page_of_native_controls_mounted() {
    let mut source = SOURCE.trim_end_matches("-> END\n").to_owned();
    for index in 0..1024 {
        source.push_str(&format!("! immediate play_sfx(sound_{index})\n"));
    }
    source.push_str("-> END\n");
    let mut fixture = open_fixture(&source).unwrap();
    let writer = &mut fixture.writer;
    assert_eq!(
        writer.rules.as_ref().unwrap().projection.effects.len(),
        1026
    );
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = view::panel(writer, writer.rules.as_ref().unwrap());
    let (pod, _) = view.build(&mut ctx, writer);
    let harness = TestHarness::create_with_size(
        crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
        NewWidget::new(SizedBox::new(pod.new_widget)),
        (800, 1600),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let (mut fields, mut nodes) = (0, 0);
    while let Some(node) = pending.pop() {
        nodes += 1;
        fields += usize::from(node.role() == accesskit::Role::MultilineTextInput);
        pending.extend(node.children());
    }
    assert_eq!(fields, 4);
    assert!(
        nodes <= 128,
        "one page measured 112 accessible nodes; got {nodes}"
    );
    println!("native_rule_nodes={nodes}; mounted_fields={fields}; destination_effects=1026");
    let target = first_argument(writer).unwrap();
    writer.edit_rule_argument(target, "5".into());
    assert!(writer.session.model().draft().contains("standing(5)"));
    assert_eq!(
        writer.rules.as_ref().unwrap().projection.effects.len(),
        1026
    );
    assert_eq!(writer.session.model().document().source(), source);
}
