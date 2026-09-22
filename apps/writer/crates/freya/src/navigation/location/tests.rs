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

#[test]
fn source_update_links_keep_the_selected_change() {
    let location: Location = "/source-updates?catalogue=locale%2Fcy-x-cofi.po&page=25"
        .parse()
        .expect("source update link");
    assert_eq!(location.screen, Screen::Updates);
    assert_eq!(location.page, 25);
    assert_eq!(location.to_string().parse::<Location>(), Ok(location));
}

#[test]
fn reply_rule_links_require_a_target_and_preserve_it_in_history() {
    let route: Location = "/reply-rules?scene=hello.recite&passage=22222222222222222222"
        .parse()
        .expect("reply route");
    assert_eq!(route.to_string().parse::<Location>(), Ok(route.clone()));
    let other = Location {
        passage: Some("33333333333333333333".into()),
        ..route.clone()
    };
    assert!(!route.same_place(&other));
    assert!("/reply-rules".parse::<Location>().is_err());
    assert!(
        "/reply-rules?passage=22222222222222222222&view=source"
            .parse::<Location>()
            .is_err()
    );
}

#[test]
fn explicit_writing_views_round_trip_and_legacy_links_remain_valid() {
    for view in ["script", "map", "source"] {
        let link = format!("/write?scene=relay_hub&view={view}");
        let parsed = link.parse::<Location>().expect("writing view");
        assert_eq!(parsed.to_string().parse::<Location>(), Ok(parsed));
    }
    assert!(
        "/write?scene=relay_hub&beat=relay_desk"
            .parse::<Location>()
            .is_ok()
    );
    assert!("/write?view=unknown".parse::<Location>().is_err());
}
