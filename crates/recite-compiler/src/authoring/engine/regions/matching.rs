use super::{CachedRegion, DocumentAnalysis, SourceRegion};

/// Match unchanged prefixes/suffixes by bytes, never by a hash alone. The
/// middle also admits unchanged regions at their original line positions.
pub(super) fn match_regions<'a>(
    sources: &[SourceRegion<'_>],
    previous: Option<&'a DocumentAnalysis>,
) -> Vec<Option<&'a CachedRegion>> {
    let mut matches = vec![None; sources.len()];
    let Some(old) = previous else {
        return matches;
    };
    let equal = |new: &SourceRegion<'_>, cached: &CachedRegion| {
        old.source.source()[cached.bytes.clone()] == *new.text()
    };
    let mut prefix = 0;
    for (new, cached) in sources.iter().zip(old.regions.iter()) {
        if !equal(new, cached) {
            break;
        }
        matches[prefix] = Some(cached);
        prefix += 1;
    }
    for (index, cached) in (prefix..sources.len())
        .rev()
        .zip(old.regions[prefix..].iter().rev())
    {
        if !equal(&sources[index], cached) {
            break;
        }
        matches[index] = Some(cached);
    }
    for (source, matched) in sources.iter().zip(&mut matches) {
        if matched.is_some() {
            continue;
        }
        if let Ok(index) = old
            .regions
            .binary_search_by_key(&source.first_line(), |r| r.first_line)
        {
            let cached = &old.regions[index];
            if equal(source, cached) {
                *matched = Some(cached);
            }
        }
    }
    matches
}
