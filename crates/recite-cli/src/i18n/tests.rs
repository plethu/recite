use super::*;

fn messages_with(
    requested: &str,
    resources: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> Messages {
    Messages::from_resources(
        requested.parse().expect("requested locale"),
        resources
            .into_iter()
            .map(|(locale, source)| (locale.parse().expect("locale"), source.to_owned())),
    )
    .expect("messages load")
}

#[test]
fn default_catalog_parses_and_contains_all_typed_messages() {
    let messages = Messages::load(&UiLocale::default()).expect("messages load");
    let default = messages
        .bundles
        .get(DEFAULT_LOCALE)
        .expect("default bundle exists");

    for id in MsgId::ALL {
        assert!(
            default.get_message(id.key()).is_some(),
            "missing {}",
            id.key()
        );
    }
}

#[test]
fn formats_messages_with_variables() {
    let messages = Messages::load(&UiLocale::default()).expect("messages load");

    assert_eq!(
        messages.format(
            MsgId::PlayStart,
            [
                ("asset", "asset-1".to_owned()),
                ("block", "start".to_owned())
            ],
        ),
        "play asset=asset-1 block=start"
    );
}

#[test]
fn missing_requested_message_falls_back_to_default_catalog() {
    let messages = messages_with(
        "en-GB",
        [
            ("en-US", DEFAULT_RESOURCE),
            ("en-GB", "other-message = Other\n"),
        ],
    );

    assert_eq!(
        messages.format(
            MsgId::PlayStart,
            [
                ("asset", "asset-1".to_owned()),
                ("block", "start".to_owned())
            ],
        ),
        "play asset=asset-1 block=start"
    );
}

#[test]
fn malformed_non_default_catalog_falls_back_to_default_catalog() {
    let messages = messages_with(
        "en-GB",
        [
            ("en-US", DEFAULT_RESOURCE),
            ("en-GB", "not valid fluent = {"),
        ],
    );

    assert_eq!(
        messages.format(
            MsgId::PlayStart,
            [
                ("asset", "asset-1".to_owned()),
                ("block", "start".to_owned())
            ],
        ),
        "play asset=asset-1 block=start"
    );
}

#[test]
fn parses_config_locale_values() {
    assert_eq!(
        UiLocale::parse("en-US").expect("locale").to_string(),
        "en-US"
    );
    assert_eq!(UiLocale::parse("system").expect("system"), UiLocale::System);
    assert!(UiLocale::parse("not a locale").is_err());
}
