//! Compare composed analysis outputs without walking byte-identical regions.
//! Shared slices are an identity proof; new slices still require exact equality.
pub(crate) fn matches<'a, T: PartialEq + 'a>(
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
