use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct Record {
    value: usize,
    clones: Arc<AtomicUsize>,
    comparisons: Arc<AtomicUsize>,
}
impl Clone for Record {
    fn clone(&self) -> Self {
        self.clones.fetch_add(1, Ordering::Relaxed);
        Self {
            value: self.value,
            clones: self.clones.clone(),
            comparisons: self.comparisons.clone(),
        }
    }
}
impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.comparisons.fetch_add(1, Ordering::Relaxed);
        self.value == other.value
    }
}

#[test]
fn ten_thousand_unchanged_records_join_without_clones_or_record_comparisons() {
    let clones = Arc::new(AtomicUsize::new(0));
    let comparisons = Arc::new(AtomicUsize::new(0));
    let previous = Arc::new(
        (0..10_000)
            .map(|value| Record {
                value,
                clones: clones.clone(),
                comparisons: comparisons.clone(),
            })
            .collect::<Vec<_>>(),
    );
    let joined = join_shared(previous.chunks(100), Some(&previous));
    assert!(Arc::ptr_eq(&joined, &previous));
    assert_eq!(clones.load(Ordering::Relaxed), 0);
    assert_eq!(comparisons.load(Ordering::Relaxed), 0);
    assert_eq!(joined[9_999].value, 9_999);

    let mut relocated = joined;
    assert!(region_mut(&mut relocated, 0..0).is_empty());
    assert_eq!(
        clones.load(Ordering::Relaxed),
        0,
        "empty regions must not clone shared families"
    );
    region_mut(&mut relocated, 0..1)[0].value = 42;
    assert_eq!(clones.load(Ordering::Relaxed), 10_000);
    assert_eq!(previous[0].value, 0);
    assert_eq!(relocated[0].value, 42);

    clones.store(0, Ordering::Relaxed);
    let changed = join_shared(relocated.chunks(100), Some(&previous));
    assert!(!Arc::ptr_eq(&changed, &previous));
    assert_eq!(clones.load(Ordering::Relaxed), 10_000);
    assert_eq!(changed[0].value, 42);
    assert_eq!(changed[9_999].value, 9_999);
    assert_eq!(previous[0].value, 0);
}
