use crate::{SourceLineIndex, SourcePosition, source_location::position_for_byte_offset};
use std::ops::Range;

use toml_edit::{Document, Item, TableLike};

#[cfg(test)]
mod tests;

/// Source ranges indexed by decoded TOML paths.
///
/// The index deliberately stores paths as TOML's decoded keys rather than
/// searching source text. This makes quoted keys, comments, reordered fields,
/// and Unicode content behave exactly like the deserialized document.
#[derive(Clone, Debug)]
pub(crate) struct TomlSpanIndex {
    entries: Vec<Entry>,
    tables: Vec<TableEntry>,
    entries_by_path: Vec<usize>,
    tables_by_path: Vec<usize>,
    lines: SourceLineIndex,
}

#[derive(Clone, Debug)]
struct Entry {
    path: Vec<String>,
    key: String,
    key_range: Option<Range<usize>>,
    value_range: Option<Range<usize>>,
    string_value: Option<String>,
    is_float: bool,
}

#[derive(Clone, Debug)]
struct TableEntry {
    path: Vec<String>,
    range: Option<Range<usize>>,
}

impl TomlSpanIndex {
    pub(crate) fn from_document<S: AsRef<str>>(document: &Document<S>) -> Self {
        let mut entries = Vec::new();
        let mut tables = Vec::new();
        collect_table(document.as_table(), &[], None, &mut entries, &mut tables);
        entries.sort_by_key(|entry| {
            entry
                .key_range
                .as_ref()
                .map_or(usize::MAX, |range| range.start)
        });
        tables.sort_by_key(|table| table.range.as_ref().map_or(usize::MAX, |range| range.start));
        let mut entries_by_path: Vec<_> = (0..entries.len()).collect();
        entries_by_path.sort_by(|&left, &right| entries[left].path.cmp(&entries[right].path));
        let mut tables_by_path: Vec<_> = (0..tables.len()).collect();
        tables_by_path.sort_by(|&left, &right| tables[left].path.cmp(&tables[right].path));
        Self {
            entries,
            tables,
            entries_by_path,
            tables_by_path,
            lines: SourceLineIndex::new(document.raw()),
        }
    }

    pub(crate) fn position(&self, offset: usize) -> SourcePosition {
        self.lines
            .position_for_byte_offset(offset)
            .unwrap_or_else(|| position_for_byte_offset(self.lines.source(), offset))
    }

    pub(crate) fn line_index(&self) -> &SourceLineIndex {
        &self.lines
    }

    fn entry(&self, path: &[String]) -> Option<&Entry> {
        let index = self
            .entries_by_path
            .partition_point(|&index| self.entries[index].path.as_slice() < path);
        let entry = &self.entries[*self.entries_by_path.get(index)?];
        (entry.path == path).then_some(entry)
    }

    pub(crate) fn key_range(&self, path: &[String]) -> Option<Range<usize>> {
        self.entry(path).and_then(|entry| entry.key_range.clone())
    }

    pub(crate) fn value_range(&self, path: &[String]) -> Option<Range<usize>> {
        self.entry(path).and_then(|entry| entry.value_range.clone())
    }

    pub(crate) fn float_range(&self, path: &[String]) -> Option<Range<usize>> {
        let index = self
            .entries_by_path
            .partition_point(|&index| self.entries[index].path.as_slice() < path);
        self.entries_by_path[index..]
            .iter()
            .map(|&index| &self.entries[index])
            .take_while(|entry| entry.path == path)
            .find(|entry| entry.is_float)
            .and_then(|entry| entry.value_range.clone())
    }

    pub(crate) fn table_range(&self, path: &[String]) -> Option<Range<usize>> {
        let index = self
            .tables_by_path
            .partition_point(|&index| self.tables[index].path.as_slice() < path);
        let table = &self.tables[*self.tables_by_path.get(index)?];
        (table.path == path).then(|| table.range.clone()).flatten()
    }

    /// Find the next exact key or string-value occurrence in a top-level
    /// section. This supports schema diagnostics that consume fields by
    /// occurrence while exact paths serve source-backed owners.
    pub(crate) fn find(
        &self,
        section: Option<&str>,
        needle: &str,
        value: bool,
        after: usize,
    ) -> Option<Range<usize>> {
        self.entries.iter().find_map(|entry| {
            if section
                .is_some_and(|section| entry.path.first().map(String::as_str) != Some(section))
                || entry
                    .key_range
                    .as_ref()
                    .is_none_or(|range| range.end <= after)
            {
                return None;
            }

            if value {
                (entry.string_value.as_deref() == Some(needle))
                    .then(|| entry.value_range.clone())
                    .flatten()
            } else {
                (entry.key == needle)
                    .then(|| entry.key_range.clone())
                    .flatten()
            }
        })
    }
}

fn collect_table(
    table: &dyn TableLike,
    parent: &[String],
    table_range: Option<Range<usize>>,
    entries: &mut Vec<Entry>,
    tables: &mut Vec<TableEntry>,
) {
    if !parent.is_empty() {
        tables.push(TableEntry {
            path: parent.to_vec(),
            range: table_range,
        });
    }

    for (key, item) in table.iter() {
        let mut path = parent.to_vec();
        path.push(key.to_owned());
        entries.push(Entry {
            path: path.clone(),
            key: key.to_owned(),
            key_range: table.key(key).and_then(|key| key.span()),
            value_range: item_value_range(item),
            string_value: item.as_str().map(str::to_owned),
            is_float: item.as_float().is_some(),
        });

        if let Some(table) = item.as_table() {
            collect_table(table, &path, table.span(), entries, tables);
        } else if let Some(table) = item.as_inline_table() {
            collect_table(table, &path, None, entries, tables);
        }

        if let Some(array) = item.as_array() {
            collect_array(array, &path, entries, tables);
        }

        if let Some(tables_array) = item.as_array_of_tables() {
            for (index, table) in tables_array.iter().enumerate() {
                let mut array_path = path.clone();
                array_path.push(format!("[{index}]"));
                collect_table(table, &array_path, table.span(), entries, tables);
            }
        }
    }
}

fn collect_array(
    array: &toml_edit::Array,
    parent: &[String],
    entries: &mut Vec<Entry>,
    tables: &mut Vec<TableEntry>,
) {
    for (index, value) in array.iter().enumerate() {
        let mut element_path = parent.to_vec();
        element_path.push(format!("[{index}]"));
        let element_range = value.span();
        entries.push(Entry {
            path: element_path.clone(),
            key: format!("[{index}]"),
            key_range: element_range.clone(),
            value_range: element_range,
            string_value: value.as_str().map(str::to_owned),
            is_float: value.as_float().is_some(),
        });
        if let Some(table) = value.as_inline_table() {
            collect_table(table, &element_path, None, entries, tables);
        }
        if let Some(array) = value.as_array() {
            collect_array(array, &element_path, entries, tables);
        }
    }
}

fn item_value_range(item: &Item) -> Option<Range<usize>> {
    item.as_value()
        .and_then(|value| value.span())
        .or_else(|| item.as_array_of_tables().and_then(|tables| tables.span()))
}
