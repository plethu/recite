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
fn mounted_declaration_list_filters_one_scrolling_list_and_keeps_complete_details() {
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
    let total = writer.workspaces.declarations.rows.len();
    let mut mounted = Mounted::new(writer, surface);
    assert!(mounted.contains_text(&format!("{total} declarations")));
    // The whole display name wraps in one named, scrolling detail area: no detail pages.
    assert!(detail_area(&mounted));
    assert!(mounted.contains_text("Actor starts"));
    assert!(mounted.contains_text("Actor ends"));
    assert!(!mounted.contains_text("More declaration detail"));
    // One list, every row mounted while it fits, and no pagers.
    // The list mounts the rows in view once it has laid out.
    mounted.settle();
    mounted.activate("actor06 · Speaker");
    assert!(mounted.is_selected("actor06 · Speaker"));
    assert!(!mounted.contains_text("Next declarations"));
    assert!(!mounted.contains_text("Previous declarations"));
    mounted.type_text("Search declarations", "actor06");
    assert!(mounted.is_selected("actor06 · Speaker"));
    assert!(mounted.contains_text(&format!("1 of {total} declarations")));
    assert!(!mounted.contains_text("actor00 · Speaker"));
    mounted.replace_text("Search declarations", "mara");
    assert!(mounted.contains_text("Mara"));
    mounted.replace_text("Search declarations", "not present");
    assert!(mounted.contains_text("No declarations match this search."));
    assert!(!mounted.contains_text("Mara"));
    mounted.replace_text("Search declarations", "actor06");
    mounted.activate("Declaration source");
    mounted.activate("Inspect declarations");
    assert!(mounted.is_selected("actor06 · Speaker"));
    assert_eq!(
        mounted.state.session.model().draft(),
        "Unapplied manuscript remains."
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("schema.json")).unwrap(),
        schema
    );
}

#[test]
fn chosen_effect_lists_its_parameters_as_names_then_types() {
    let directory = fixture().unwrap();
    fs::write(
        directory.path().join("schema.json"),
        r#"{"schema_version":1,"speakers":{"mara":{"display_name":"Mara"}},"effects":{"change_disposition":{"modes":["immediate"],"params":[{"name":"speaker","type":"speaker"},{"name":"delta","type":"int"}]}}}"#,
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Declarations)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    // The list mounts the rows in view once it has laid out.
    mounted.settle();
    mounted.activate("change_disposition · Effect request");
    assert!(mounted.contains_text("Parameters"));
    // Labels in reading order: a depth-first walk in child order.
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut texts = Vec::new();
    while let Some(node) = pending.pop() {
        texts.extend(node.label().or_else(|| node.value()));
        pending.extend(node.children().rev());
    }
    let list = texts
        .iter()
        .position(|label| label == "Parameters")
        .unwrap();
    let order = ["speaker", "Speaker", "delta", "Integer"].map(|text| {
        texts[list..]
            .iter()
            .position(|label| label == text)
            .unwrap()
    });
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{texts:?}");
    mounted.activate("mara · Speaker");
    assert!(!mounted.contains_text("Parameters"));
    assert!(mounted.contains_text("Mara"));
}

#[test]
fn an_unbroken_display_name_sits_on_one_line_named_in_full() {
    let directory = fixture().unwrap();
    let token = "relay_station_announcer_".repeat(40);
    fs::write(
        directory.path().join("schema.json"),
        format!(
            "{{\"schema_version\":1,\"speakers\":{{\"mara\":{{\"display_name\":\"{token}\"}}}}}}"
        ),
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Declarations)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    // The list mounts the rows in view once it has laid out.
    mounted.settle();
    mounted.activate("mara · Speaker");
    // Nothing to wrap or scroll: the ellipsised line still reads the whole token aloud.
    assert!(mounted.contains_text(&token));
    assert!(!detail_area(&mounted));
}

/// Whether the chosen declaration's prose sits in its named, scrolling detail area.
fn detail_area(mounted: &Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>) -> bool {
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.label().as_deref() == Some("Declaration detail") {
            return true;
        }
        pending.extend(node.children());
    }
    false
}
