use super::*;
#[test]
fn a_shrunken_projection_normalizes_to_a_full_last_range() {
    let mut window = ReadingWindow::default();
    window.reveal(1000);
    window.normalize(20);
    assert_eq!(window.range(20), 16..20);
    window.normalize(10);
    assert_eq!(window.range(10), 0..10);
    window.normalize(0);
    assert!(window.range(0).is_empty());
}
#[test]
fn steps_overlap_with_a_fixed_budget_and_jumps_reveal_only_when_needed() {
    let first = ReadingWindow::default();
    assert!(first.shifted(Direction::Previous, 41).is_none());
    let second = first.shifted(Direction::Next, 41).unwrap();
    assert_eq!(second.range(41), 8..24);
    let third = second.shifted(Direction::Next, 41).unwrap();
    assert_eq!(third.range(41), 16..32);
    assert_eq!(third.shifted(Direction::Previous, 41), Some(second));
    let mut window = second;
    window.reveal(20);
    assert_eq!(window, second);
    window.reveal(39);
    assert_eq!(window.range(41), 32..41);
    assert!(!window.is_shift());
    assert!(window.shifted(Direction::Next, 41).is_none());
    for total in [0, 1, 16, 17, 10_001] {
        let mut window = first;
        while let Some(next) = window.shifted(Direction::Next, total) {
            assert!(next.range(total).len() <= CAPACITY);
            assert!(next.range(total).start < window.range(total).end);
            window = next;
        }
        assert_eq!(window.range(total).end, total);
    }
}
