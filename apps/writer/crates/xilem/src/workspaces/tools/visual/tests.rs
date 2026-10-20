use super::*;
use crate::presentation::Theme;
use std::sync::Arc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{core::Widget, testing::TestHarness},
};

fn capture_action<W: Widget>(harness: &mut TestHarness<W>, prefix: &str) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    let target = loop {
        let widget = pending.pop().unwrap();
        if support::native_label(harness, widget.id()).as_deref() == Some("Regenerate schema") {
            break widget.id();
        }
        pending.extend(widget.children());
    };
    harness.scroll_into_view(target);
    harness
        .render()
        .save(format!("{prefix}-producer-200-ready-action.png"))
        .unwrap();
}

#[test]
fn producer_command_and_busy_states_render_at_personal_scales() {
    for scale in [100, 200] {
        for busy in [false, true] {
            let (_directory, mut writer, _jobs) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.show_declaration_producer();
            if busy {
                writer.regenerate_schema();
            }
            let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
            let mut context = ViewCtx::new(Arc::new(support::NoProxy), runtime);
            let view = crate::view::writer(&writer);
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with_size(
                Theme::new(&writer.config.writer).initial_properties(),
                pod.new_widget,
                (1280, 800),
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_GENERATION") {
                harness
                    .render()
                    .save(format!(
                        "{prefix}-producer-{scale}-{}.png",
                        if busy { "busy" } else { "ready" }
                    ))
                    .unwrap();
                if scale == 200 && !busy {
                    capture_action(&mut harness, &prefix);
                }
            }
        }
    }
}
