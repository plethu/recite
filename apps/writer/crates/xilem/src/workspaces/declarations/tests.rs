#![allow(unused_imports)]
use super::super::*;
use super::*;
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
    workspaces::{Open, opening, tests::fixture},
};
use std::fs;
use xilem::masonry::widgets::Passthrough;

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_declaration_search_pages_and_complete_details_preserve_the_manuscript() {
    let directory = fixture().unwrap();
    let name = format!(
        "Actor starts {} Actor ends",
        "a detailed display name ".repeat(100)
    );
    let speakers = (0..7)
        .map(|index| format!("\"actor{index:02}\":{{\"display_name\":\"{name}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    let schema = format!(
        "{{\"schema_version\":1,\"speakers\":{{{speakers},\"mara\":{{\"display_name\":\"Mara\"}}}}}}"
    );
    fs::write(directory.path().join("schema.json"), &schema).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Declarations)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    writer.change(|model| {
        model.set_draft("Unapplied manuscript remains.".into());
        Ok(())
    });
    let mut mounted = Mounted::new(writer, surface);
    mounted.activate("More declaration detail");
    assert!(!mounted.contains_text("Actor starts"));
    mounted.activate("Previous declaration detail");
    assert!(mounted.contains_text("Actor starts"));
    mounted.activate("Next declarations");
    mounted.activate("Declaration · actor06 · Speaker");
    mounted.activate("Previous declarations");
    mounted.type_text("Search declarations", "actor06");
    assert!(mounted.contains_text("Selected · actor06 · Speaker"));
    assert!(!mounted.contains_text("actor00 · Speaker"));
    assert!(!mounted.contains_text("Next declarations"));
    assert!(!mounted.contains_text("Previous declarations"));
    mounted.replace_text("Search declarations", "mara");
    assert!(mounted.contains_text("Mara"));
    assert!(!mounted.contains_text("More declaration detail"));
    assert!(!mounted.contains_text("Previous declaration detail"));
    mounted.replace_text("Search declarations", "not present");
    assert!(mounted.contains_text("No declarations match this search."));
    assert!(!mounted.contains_text("Mara"));
    mounted.replace_text("Search declarations", "actor06");
    mounted.activate("Declaration source");
    mounted.activate("Inspect declarations");
    assert!(mounted.contains_text("Selected · actor06 · Speaker"));
    assert_eq!(
        mounted.state.session.model().draft(),
        "Unapplied manuscript remains."
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("schema.json")).unwrap(),
        schema
    );
}
