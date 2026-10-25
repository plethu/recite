use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::{
    masonry::{
        accesskit::{Action, Live, Role},
        properties::TextAreaLabel,
        widgets::Passthrough,
    },
    view::{flex_col, text_button, text_input},
};
struct State {
    name: &'static str,
    text: String,
    invoked: bool,
}
fn page(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    keyed(
        (),
        named(
            state.name,
            flex_col((
                text_input(state.text.clone(), |state: &mut State, text| {
                    state.text = text
                })
                .prop(TextAreaLabel::new("Owned field")),
                text_button("Owned action", |state: &mut State| state.invoked = true),
            )),
        ),
    )
}
#[test]
fn named_groups_expose_their_children_and_preserve_native_controls_when_renamed() {
    let mut mounted = Mounted::new(
        State {
            name: "Preview pane",
            text: "Original".into(),
            invoked: false,
        },
        page,
    );
    let field = mounted.find("Owned field");
    mounted.replace_text("Owned field", "Retained draft");
    mounted.state.name = "Conversation preview";
    mounted.settle();
    assert_eq!(mounted.find("Owned field"), field);
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let group = loop {
        let node = pending.pop().expect("named native group");
        if node.role() == Role::Group && node.label().as_deref() == Some("Conversation preview") {
            break node;
        }
        pending.extend(node.children());
    };
    assert!(
        !group.data().supports_action(Action::Focus),
        "the group does not add a keyboard stop"
    );
    assert!(group.children().next().is_some());
    mounted.activate("Owned action");
    assert!(mounted.state.invoked);
    assert_eq!(mounted.state.text, "Retained draft");
}

fn status_page(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    keyed((), status(state.name, flex_col(())))
}

#[test]
fn changing_status_is_a_polite_native_live_region_without_a_keyboard_stop() {
    let mut mounted = Mounted::new(
        State {
            name: "Preview failed.",
            text: String::new(),
            invoked: false,
        },
        status_page,
    );
    mounted.state.name = "Preparation cancelled.";
    mounted.settle();
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let node = loop {
        let node = pending.pop().expect("native status region");
        if node.role() == Role::Status {
            break node;
        }
        pending.extend(node.children());
    };
    assert_eq!(node.label().as_deref(), Some("Preparation cancelled."));
    assert_eq!(node.data().live(), Some(Live::Polite));
    assert!(!node.data().supports_action(Action::Focus));
}
