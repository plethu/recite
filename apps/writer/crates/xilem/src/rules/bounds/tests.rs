use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::accesskit;
use xilem::{
    ViewCtx,
    core::View,
    masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
};

fn navigate_after_render(
    writer: &mut Writer,
    runtime: &tokio::runtime::Runtime,
    location: Location,
) {
    // Building a view installs its own worker channels; restore the owned fixture driver.
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.navigate(Target::Location(location));
    support::finish_navigation(writer, &mut jobs).unwrap();
}

#[test]
fn long_beat_controls_do_not_leak_into_reply_rules_or_source_repair() {
    let lines = (100..132)
        .map(|n| format!("> line_{n}@{n:020x}\n  A long beat passage.\n"))
        .collect::<String>();
    let source = SOURCE.replacen("? reply@", &format!("{lines}? reply@"), 1);
    let mut fixture = open_fixture(&source).unwrap();
    let writer = &mut fixture.writer;
    assert!(writer.reading.rows.len() > crate::reading::PAGE);
    let labels = support::labels(writer).unwrap();
    assert!(labels.iter().any(|text| text == "Reply rules"));
    assert!(!labels.iter().any(|text| text == "Find in this beat"));
    assert!(!labels.iter().any(|text| text.starts_with("Entries ")));
    navigate_after_render(
        writer,
        &fixture._runtime,
        Location {
            beat: Some("start".into()),
            ..Location::default()
        },
    );
    assert_eq!(
        writer.session.model().view(),
        &recite_writer_model::View::Block("start".into()),
        "{}",
        writer.status
    );
    let labels = support::labels(writer).unwrap();
    assert!(
        labels.iter().any(|text| text == "Find in this beat"),
        "rows={}, selected={:?}, labels={labels:?}",
        writer.reading.rows.len(),
        writer.reading.selected
    );
    // Recovery can restore Source selection; an explicit source link uses the IDE instead.
    writer.change(|model| model.select(recite_writer_model::View::Source));
    let labels = support::labels(writer).unwrap();
    assert!(
        labels
            .iter()
            .any(|text| text.starts_with("Choose a beat in the outline"))
    );
    assert!(!labels.iter().any(|text| text == "Find in this beat"));
    assert!(!labels.iter().any(|text| text.starts_with("Entries ")));
}

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
