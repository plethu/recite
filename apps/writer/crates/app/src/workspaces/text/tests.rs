use super::*;

#[test]
fn reading_pages_preserve_words_and_all_unicode_even_when_one_word_exceeds_the_bound() {
    let prose = "À la gare, le dernier tram attend. ".repeat(600);
    let reading = reading_pages(&prose);
    assert_eq!(reading.concat(), prose);
    assert!(reading.iter().all(|page| page.chars().count() <= 1_000));
    assert!(
        reading
            .iter()
            .all(|page| page.ends_with(char::is_whitespace))
    );
    for source in [String::new(), "é".repeat(12_000), "a ".repeat(5_000)] {
        let reading = reading_pages(&source);
        assert_eq!(reading.concat(), source);
        assert!(
            reading
                .iter()
                .all(|page| !page.is_empty() && page.chars().count() <= 1_000)
        );
    }
}

#[test]
fn a_shown_row_cuts_each_side_to_one_diff_row_and_marks_the_cut() {
    let row = shown_row(ComparisonRow {
        caption: "Line 1".into(),
        before: Some("駅".repeat(1_500)),
        after: Some("short".into()),
    });
    assert_eq!(
        row.before.as_deref().map(|text| text.chars().count()),
        Some(1_001)
    );
    assert!(
        row.before
            .as_deref()
            .is_some_and(|text| text.ends_with('…'))
    );
    assert_eq!(row.after.as_deref(), Some("short"));
}
