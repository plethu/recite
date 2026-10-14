//! One section's decoded strings, preserving role-specific source occurrences.
use super::{SourceRange, StringRole, skip_json_string, string_role_matches};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct JsonStringIndex {
    values: BTreeMap<String, Vec<Occurrence>>,
}

#[derive(Debug)]
struct Occurrence {
    range: SourceRange,
    is_key: bool,
    top_key: bool,
}

impl JsonStringIndex {
    pub(super) fn new(source: &str, range: SourceRange) -> Self {
        let mut values: BTreeMap<String, Vec<Occurrence>> = BTreeMap::new();
        for (range, depth) in strings(source, range) {
            let Ok(value) = serde_json::from_str::<String>(&source[range.start..range.end]) else {
                continue;
            };
            values.entry(value).or_default().push(Occurrence {
                range,
                is_key: string_role_matches(source, range.end, StringRole::AnyKey, depth),
                top_key: string_role_matches(source, range.end, StringRole::Key, depth),
            });
        }
        Self { values }
    }

    pub(super) fn find(&self, needle: &str, role: StringRole, after: usize) -> Option<SourceRange> {
        let values = self.values.get(needle)?;
        let start = values.partition_point(|value| value.range.start < after);
        values[start..]
            .iter()
            .find(|value| match role {
                StringRole::Key => value.top_key,
                StringRole::AnyKey => value.is_key,
                StringRole::Value => !value.is_key,
            })
            .map(|value| value.range)
    }
}

pub(super) fn strings(
    source: &str,
    range: SourceRange,
) -> impl Iterator<Item = (SourceRange, usize)> + '_ {
    let bytes = source.as_bytes();
    let mut index = range.start;
    let mut depth = 0usize;
    let mut finished = false;
    std::iter::from_fn(move || {
        if finished {
            return None;
        }
        while index < range.end {
            match bytes[index] {
                b'"' => {
                    let start = index;
                    let Some(end) = skip_json_string(bytes, start).filter(|end| *end < range.end)
                    else {
                        finished = true;
                        return None;
                    };
                    index = end + 1;
                    return Some((
                        SourceRange {
                            start,
                            end: end + 1,
                        },
                        depth,
                    ));
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    let Some(next) = depth.checked_sub(1) else {
                        finished = true;
                        return None;
                    };
                    depth = next;
                }
                _ => {}
            }
            index += 1;
        }
        finished = true;
        None
    })
}

#[cfg(test)]
mod tests;
