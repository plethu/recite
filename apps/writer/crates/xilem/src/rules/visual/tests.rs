use super::*;
use crate::tests::support::NoProxy;
use xilem::{
    ViewCtx,
    core::View,
    masonry::{
        core::{NewWidget, WidgetId},
        properties::TextAreaLabel,
        testing::TestHarness,
        widgets::{SizedBox, TextInput},
    },
};

fn input(harness: &TestHarness<SizedBox>, owner: &str) -> Option<WidgetId> {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if let Some(input) = widget.downcast::<TextInput>()
            && input.get_prop::<TextAreaLabel>().label.starts_with(owner)
        {
            return Some(input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}

#[test]
fn rule_fields_and_expanded_actions_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in ["controls", "condition-actions", "effect-actions", "invalid"] {
            let mut fixture = fixture().unwrap();
            let writer = &mut fixture.writer;
            let mut config = writer.config.clone();
            config.writer.presentation = config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.configure(config);
            let target = first_argument(writer).unwrap();
            match stage {
                "condition-actions" => writer.rule_controls.expanded = Some(target.call.clone()),
                "effect-actions" => writer.rule_controls.expanded = Some(CallTarget::Effect(0)),
                "invalid" => writer.edit_rule_argument(target, "-".into()),
                _ => {}
            }
            let mut ctx = ViewCtx::new(
                Arc::new(NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let view = crate::view::writer(writer);
            let (pod, _) = view.build(&mut ctx, writer);
            let mut harness = TestHarness::create_with_size(
                crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
                NewWidget::new(SizedBox::new(pod.new_widget)),
                (1280, 1000),
            );
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let mut fields = 0;
            while let Some(node) = pending.pop() {
                fields +=
                    usize::from(node.role() == xilem::masonry::accesskit::Role::MultilineTextInput);
                pending.extend(node.children());
            }
            assert!(fields > 0 && fields <= 8);
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_RULES") {
                harness
                    .render()
                    .save(format!("{prefix}-{stage}-{scale}.png"))
                    .unwrap();
                let owner = if stage == "effect-actions" {
                    "play_sfx"
                } else {
                    "standing"
                };
                harness.scroll_into_view(input(&harness, owner).unwrap());
                harness
                    .render()
                    .save(format!("{prefix}-{stage}-scrolled-{scale}.png"))
                    .unwrap();
            }
        }
    }
}
