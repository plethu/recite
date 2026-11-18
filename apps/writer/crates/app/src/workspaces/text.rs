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
/// The most of one text a diff row shows; the file keeps all of it.
const SHOWN_CHARS: usize = 1_000;
/// `row` with each side cut to what one diff row shows, an ellipsis marking the cut.
pub(crate) fn shown_row(row: ComparisonRow) -> ComparisonRow {
    let shown = |text: String| {
        if text.chars().count() <= SHOWN_CHARS {
            text
        } else {
            let mut shown: String = text.chars().take(SHOWN_CHARS).collect();
            shown.push('…');
            shown
        }
    };
    ComparisonRow {
        caption: row.caption,
        before: row.before.map(shown),
        after: row.after.map(shown),
    }
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
