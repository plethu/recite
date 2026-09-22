use super::range;
#[test]
fn highlights_changed_words_without_splitting_unicode() {
    let old = "À neuf le tram part.";
    let new = "À huit le tram part.";
    assert_eq!(&old[range(old, new)], "neuf ");
    assert_eq!(&new[range(new, old)], "huit ");
    assert!(range(old, old).is_empty());
    assert_eq!(&new[range(new, "")], new);
}
