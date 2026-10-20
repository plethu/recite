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
        assert!(
            matches(query).iter().any(|choice| choice.value == code),
            "{query}"
        );
    }
}

#[test]
fn cofi_is_discoverable_by_name_and_tag_but_not_in_the_default_list() {
    for query in ["Cofi", "cofi", "cy-x-cofi"] {
        let choices = matches(query);
        assert_eq!(
            choices
                .iter()
                .filter(|choice| choice.value == "cy-x-cofi")
                .count(),
            1
        );
        assert!(choices.iter().any(|choice| choice.title == "Welsh (Cofi)"));
    }
    assert!(!matches("").iter().any(|choice| choice.value == "cy-x-cofi"));
    assert!(caption("en-x-pirate").is_none());
}

#[test]
fn results_are_complete_unique_and_exact_tags_take_priority() {
    for query in ["", "a", "English", "FR-ca", "cy-x-cofi", "日本語"] {
        let choices = matches(query);

        let tags: std::collections::BTreeSet<_> =
            choices.iter().map(|choice| &choice.value).collect();
        assert_eq!(tags.len(), choices.len());
    }
    assert!(matches("").is_empty());
    assert!(matches("a").len() > 8);
    assert_eq!(matches("  FR-ca  ")[0].value, "fr-CA");
    assert!(matches("日本語").iter().any(|choice| &choice.value == "ja"));
    assert!(
        matches("FRANÇAIS")
            .iter()
            .any(|choice| &choice.value == "fr")
    );
}
