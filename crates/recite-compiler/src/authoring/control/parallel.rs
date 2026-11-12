//! Ordered fan-out of independent per-document work under one cooperative control.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use super::{Interrupted, WorkControl};

/// Below this many input bytes, thread start-up costs more than it saves.
const PARALLEL_MIN_BYTES: usize = 128 * 1024;
const MAX_WORKERS: usize = 8;

/// Applies `work` to every item and returns the results in item order.
///
/// Items run on scoped worker threads when there are at least two and their
/// combined `bytes` reach a threshold; otherwise they run in order on the
/// caller's thread. Results never depend on scheduling. The first
/// interruption stops the remaining items and is returned; no partial result
/// is published.
pub fn map_in_order<T, R>(
    items: &[T],
    bytes: impl Fn(&T) -> usize,
    control: &dyn WorkControl,
    work: impl Fn(&T) -> Result<R, Interrupted> + Sync,
) -> Result<Vec<R>, Interrupted>
where
    T: Sync,
    R: Send,
{
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_WORKERS)
        .min(items.len());
    if workers < 2 || items.iter().map(bytes).sum::<usize>() < PARALLEL_MIN_BYTES {
        return items.iter().map(&work).collect();
    }
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let run = || {
        let mut done = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            if index >= items.len() || stopped.load(Ordering::Relaxed) {
                return Ok(done);
            }
            match work(&items[index]) {
                Ok(result) => done.push((index, result)),
                Err(interrupted) => {
                    stopped.store(true, Ordering::Relaxed);
                    return Err(interrupted);
                }
            }
        }
    };
    let finished: Vec<Result<Vec<(usize, R)>, Interrupted>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..workers).map(|_| scope.spawn(run)).collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
            })
            .collect()
    });
    let mut indexed = Vec::with_capacity(items.len());
    for done in finished {
        indexed.extend(done?);
    }
    control.checkpoint()?;
    indexed.sort_unstable_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, result)| result).collect())
}

#[cfg(test)]
mod tests;
