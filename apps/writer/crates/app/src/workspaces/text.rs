//! Bound mounted text even when an upstream description or build report has one enormous line.
#[cfg(test)]
mod tests;
use recite_writer_session::comparison::ComparisonRow;
pub(crate) fn change_pages(row: ComparisonRow) -> Vec<ComparisonRow> {
    let before = row.before.as_deref().map(pages).unwrap_or_default();
    let after = row.after.as_deref().map(pages).unwrap_or_default();
    let count = before.len().max(after.len());
    (0..count)
        .map(|index| ComparisonRow {
            caption: if count == 1 {
                row.caption.clone()
            } else {
                format!("{} · part {} of {count}", row.caption, index + 1)
            },
            before: before.get(index).cloned(),
            after: after.get(index).cloned(),
        })
        .collect()
}
pub fn pages(text: &str) -> Vec<String> {
    let mut pages = Vec::new();
    let mut page = String::new();
    let mut count = 0;
    for character in text.chars() {
        if count == 1_000 {
            pages.push(std::mem::take(&mut page));
            count = 0;
        }
        page.push(character);
        count += 1;
    }
    if !page.is_empty() {
        pages.push(page);
    }
    pages
}

/// Reading pages preserve all text while preferring complete words at each boundary.
pub(crate) fn reading_pages(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut page = String::new();
    let mut length = 0;
    for word in text.split_inclusive(char::is_whitespace) {
        let size = word.chars().count();
        if length + size > 1_000 && !page.is_empty() {
            result.push(std::mem::take(&mut page));
            length = 0;
        }
        if size > 1_000 {
            let mut chunks = pages(word);
            page = chunks.pop().unwrap_or_default();
            length = page.chars().count();
            result.extend(chunks);
        } else {
            page.push_str(word);
            length += size;
        }
    }
    if !page.is_empty() {
        result.push(page);
    }
    result
}
