use super::*;

#[test]
fn links_round_trip_unicode_paths_and_queue_state() {
    let location = Location {
        screen: Screen::Translations,
        document: "scenes/a & b.recite".into(),
        beat: Some("arrival".into()),
        passage: Some("11111111111111111111".into()),
        catalogue: Some("locale/fr-CA.po".into()),
        query: "bonjour & café?".into(),
        attention: true,
        page: 2,
        ..Location::default()
    };
    let relative = location.to_string();
    assert_eq!(relative.parse::<Location>(), Ok(location.clone()));
    assert_eq!(
        format!("recite://writer{relative}").parse::<Location>(),
        Ok(location)
    );
}

#[test]
fn malformed_or_ambiguous_links_are_rejected() {
    for link in [
        "https://writer/write",
        "recite://other/write",
        "//other/write",
        "/unknown",
        "/write?scene=a&scene=b",
        "/write?view=source&beat=hello",
        "/translations?page=-1",
        "/write?unknown=thing",
        "/write#fragment",
    ] {
        assert!(link.parse::<Location>().is_err(), "{link}");
    }
}
