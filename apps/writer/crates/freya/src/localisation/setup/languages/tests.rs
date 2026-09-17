use super::*;
#[test]
fn language_search_recognises_names_native_names_and_regions() {
    assert!(matches("burger").is_empty());
    for (query, code) in [
        ("French", "fr"),
        ("français", "fr"),
        ("fr-ca", "fr-CA"),
        ("Brazil", "pt-BR"),
        ("Simplified", "zh-Hans"),
    ] {
        assert!(matches(query).iter().any(|(tag, _)| tag == code), "{query}");
    }
}

#[test]
fn cofi_is_discoverable_by_name_and_tag_but_not_in_the_default_list() {
    for query in ["Cofi", "cofi", "cy-x-cofi"] {
        let choices = matches(query);
        assert_eq!(
            choices.iter().filter(|(tag, _)| tag == "cy-x-cofi").count(),
            1
        );
        assert!(
            choices
                .iter()
                .any(|(_, name)| name == "Welsh (Cofi) · cy-x-cofi")
        );
    }
    assert!(!matches("").iter().any(|(tag, _)| tag == "cy-x-cofi"));
    assert!(caption("en-x-pirate").is_none());
}
