//! Compose analysis outputs and compare them without walking identical regions.
//! Shared slices are an identity proof; new slices still require exact equality.
use std::ops::Range;
use std::sync::Arc;

pub(crate) fn join_shared<'a, T: Clone + PartialEq + 'a>(
    parts: impl Iterator<Item = &'a [T]> + Clone,
    previous: Option<&Arc<Vec<T>>>,
) -> Arc<Vec<T>> {
    if let Some(previous) = previous
        && matches(parts.clone(), previous)
    {
        Arc::clone(previous)
    } else {
        Arc::new(collect(parts))
    }
}

pub(crate) fn region_mut<T: Clone>(items: &mut Arc<Vec<T>>, range: Range<usize>) -> &mut [T] {
    if range.start == range.end && range.end <= items.len() {
        &mut []
    } else {
        &mut Arc::make_mut(items)[range]
    }
}

fn matches<'a, T: PartialEq + 'a>(
    parts: impl Iterator<Item = &'a [T]>,
    mut previous: &[T],
) -> bool {
    for part in parts {
        let Some((expected, rest)) = previous.split_at_checked(part.len()) else {
            return false;
        };
        if !std::ptr::eq(part, expected) && part != expected {
            return false;
        }
        previous = rest;
    }
    previous.is_empty()
}

/// Compose slices with one output allocation instead of geometric growth.
fn collect<'a, T: Clone + 'a>(parts: impl Iterator<Item = &'a [T]> + Clone) -> Vec<T> {
    let capacity = parts.clone().map(|part| part.len()).sum();
    let mut output = Vec::with_capacity(capacity);
    for part in parts {
        output.extend_from_slice(part);
    }
    output
}

#[cfg(test)]
mod tests;
