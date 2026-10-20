//! Repeat navigation preserves discovery order and advances from an accepted semantic target.

/// Wrap through unique targets; a retired target resumes at the first or last current match.
pub fn repeat_target<'a, T: PartialEq>(
    matches: impl IntoIterator<Item = &'a T>,
    accepted: Option<&T>,
    forward: bool,
) -> Option<&'a T> {
    let mut first = None;
    let mut last = None;
    let mut previous = None;
    let mut next = None;
    let mut found = false;
    for target in matches {
        first.get_or_insert(target);
        if accepted == Some(target) {
            previous = last;
            found = true;
        } else if found && next.is_none() {
            next = Some(target);
        }
        last = Some(target);
    }
    if forward {
        next.or(first)
    } else if found {
        previous.or(last)
    } else {
        last
    }
}

#[cfg(test)]
mod tests;
