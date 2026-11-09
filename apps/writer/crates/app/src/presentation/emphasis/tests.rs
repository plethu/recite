use super::*;

#[test]
fn search_emphasis_keeps_unicode_expansions_whole_and_marks_each_term() {
    let text = "İstanbul café 🚋 café";
    let found = ranges(text, "i CAFÉ 🚋");
    assert_eq!(
        found
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<Vec<_>>(),
        ["İ", "café", "🚋", "café"]
    );
    assert!(ranges(text, "").is_empty());
    let overlapping = ranges("courier", "cour courier");
    assert_eq!(overlapping.len(), 1);
    assert_eq!(overlapping[0], 0..7);
}
