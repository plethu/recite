use super::*;
use crate::authoring::CancellationToken;
use std::sync::Mutex;

#[test]
fn large_inputs_keep_item_order() -> Result<(), Interrupted> {
    let items: Vec<usize> = (0..64).collect();
    let threads = Mutex::new(Vec::new());
    let results = map_in_order(
        &items,
        |_| 64 * 1024,
        &CancellationToken::new(),
        |item| {
            if let Ok(mut seen) = threads.lock() {
                let id = thread::current().id();
                if !seen.contains(&id) {
                    seen.push(id);
                }
            }
            Ok(item * 2)
        },
    )?;
    assert_eq!(
        results,
        items.iter().map(|item| item * 2).collect::<Vec<_>>()
    );
    let workers = thread::available_parallelism().map_or(1, usize::from);
    let seen = threads.lock().map(|seen| seen.len()).unwrap_or_default();
    assert!(seen >= 1 && seen <= workers.min(MAX_WORKERS));
    Ok(())
}

#[test]
fn small_inputs_stay_on_the_calling_thread() -> Result<(), Interrupted> {
    let caller = thread::current().id();
    let results = map_in_order(
        &[1, 2, 3],
        |_| 16,
        &CancellationToken::new(),
        |item| {
            assert_eq!(thread::current().id(), caller);
            Ok(*item)
        },
    )?;
    assert_eq!(results, [1, 2, 3]);
    Ok(())
}

#[test]
fn an_interrupted_item_publishes_no_results() {
    let items: Vec<usize> = (0..64).collect();
    let control = CancellationToken::new();
    let result = map_in_order(
        &items,
        |_| 64 * 1024,
        &control,
        |item| {
            if *item == 40 {
                control.interrupt();
            }
            control.checkpoint().map(|()| *item)
        },
    );
    assert_eq!(result, Err(Interrupted));
}
