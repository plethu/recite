use super::{Distance, closest};

#[test]
fn typos_find_their_name_and_distant_names_find_nothing() {
    let blocks = ["arrival", "arrival_return", "effects", "objects"];
    assert_eq!(closest("arrivl", blocks), Some("arrival"));
    assert_eq!(closest("effectz", blocks), Some("effects"));
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
    assert_eq!(Distance::to("elise").from("élise"), 1);
    assert_eq!(Distance::to("abc").from(""), 3);
    let mut distance = Distance::to("kitten");
    assert_eq!(
        distance.from("sitting"),
        3,
        "rows are reset between candidates"
    );
    assert_eq!(distance.from("kitten"), 0);
}

#[test]
fn length_alone_rules_out_distant_names() {
    let blocks = ["beat_0001", "beat_0002", "beat"];
    assert_eq!(closest("beat_", blocks), Some("beat"));
}
