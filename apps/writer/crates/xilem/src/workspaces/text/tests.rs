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
