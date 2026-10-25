use super::*;
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use xilem::{
    masonry::{
        core::{
            TextEvent, WidgetId,
            keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers},
        },
        layout::AsUnit,
        properties::TextAreaLabel,
        testing::TestHarness,
        widgets::{Flex, Passthrough},
    },
    style::Style,
    view::{flex_col, portal, text_input},
};

fn make(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed(
        (),
        follow_reading(
            writer,
            portal(
                flex_col((text_input(
                    writer.session.model().draft().to_owned(),
                    |writer: &mut Writer, text| {
                        writer.change(|model| {
                            model.set_draft(text);
                            Ok(())
                        });
                    },
                )
                .prop(TextAreaLabel::new("Retained editor")),))
                .dims((600.px(), 2000.px())),
            )
            .constrain_horizontal(true),
        ),
    )
}
fn viewport(harness: &TestHarness<Passthrough>) -> WidgetId {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if widget.downcast::<Portal<Flex>>().is_some() {
            return widget.id();
        }
        pending.extend(widget.children());
    }
    panic!("native reading viewport");
}
fn position(harness: &TestHarness<Passthrough>, id: WidgetId) -> Point {
    harness
        .get_widget_with_id(id)
        .downcast::<Portal<Flex>>()
        .unwrap()
        .get_viewport_pos()
}

#[test]
fn page_reset_retains_the_native_editor_and_undo_while_other_rebuilds_keep_scroll() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let writer = support::open(&directory).unwrap();
    let original = writer.session.model().draft().to_owned();
    let mut mounted = Mounted::new(writer, make);
    mounted.type_text("Retained editor", "x");
    let field = mounted.find("Retained editor");
    let portal = viewport(&mounted.harness);
    mounted.harness.edit_widget_with_id(portal, |mut widget| {
        Portal::set_viewport_pos(&mut widget.downcast::<Portal<Flex>>(), Point::new(0., 700.));
    });
    assert!(position(&mounted.harness, portal).y > 600.);
    mounted.settle();
    assert!(
        position(&mounted.harness, portal).y > 600.,
        "ordinary rebuild retains browsing position"
    );
    mounted.state.reading.window.reveal(32);
    mounted.settle();
    assert_eq!(position(&mounted.harness, portal), Point::ORIGIN);
    assert_eq!(mounted.find("Retained editor"), field);
    mounted
        .harness
        .process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character("z".into()),
            code: Code::KeyZ,
            modifiers: if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            },
            ..KeyboardEvent::default()
        }));
    mounted.settle();
    assert_eq!(mounted.state.session.model().draft(), original);
}
