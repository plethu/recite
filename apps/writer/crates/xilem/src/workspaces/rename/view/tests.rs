use super::*;
use crate::tests::support::{mounted::Mounted, open, project};
use xilem::masonry::{accesskit::Role, widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), crate::view::writer(writer))
}

#[test]
fn a_clash_is_an_error_under_its_invalid_field_and_describes_review() {
    let directory = project(":: start default\n-> old\n:: old\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.viewport.width = 1200.;
    writer.workspaces.screen = crate::workspaces::Screen::Rename;
    writer.change_rename_block("old".into());
    writer.change_rename_name("start".into());
    let mounted = Mounted::new(writer, surface);
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let (mut field, mut error, mut review) = (None, None, None);
    while let Some(node) = pending.pop() {
        let name = node.label().or_else(|| node.value()).unwrap_or_default();
        match node.role() {
            Role::TextInput if name == "New beat name" => field = Some(node.data().invalid()),
            Role::Label if name.starts_with("Error: ") => error = Some(name),
            Role::Button if name == "Review project rename" => {
                review = Some((node.is_disabled(), node.description()));
            }
            _ => {}
        }
        pending.extend(node.children());
    }
    assert!(
        field.expect("the new id's field").is_some(),
        "marked invalid"
    );
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.starts_with("Error: A beat named start already exists")),
        "{error:?}"
    );
    // Review waits, and says why in the words set under the field.
    assert_eq!(
        review,
        Some((
            true,
            Some("Choose an id no other beat in this file has".to_owned())
        ))
    );
    assert!(mounted.contains_text("Choose an id no other beat in this file has"));
}
