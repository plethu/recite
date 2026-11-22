use super::{closest, edit_distance};

#[test]
fn typos_find_their_name_and_distant_names_find_nothing() {
    let blocks = ["arrival", "arrival_return", "effects", "objects"];
    assert_eq!(closest("arival", blocks), Some("arrival"));
    assert_eq!(closest("efects", blocks), Some("effects"));
    assert_eq!(closest("station", blocks), None);
    assert_eq!(
        closest("arrival", blocks),
        None,
        "an exact name is not a suggestion"
    );
}

#[test]
fn ties_resolve_by_name_order() {
    assert_eq!(closest("cat", ["cot", "cut", "bat"]), Some("bat"));
}

#[test]
fn distance_counts_characters_not_bytes() {
    assert_eq!(edit_distance("élise", "elise"), 1);
    assert_eq!(edit_distance("", "abc"), 3);
}
