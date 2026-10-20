//! Bound mounted text even when an upstream description or build report has one enormous line.
pub(super) fn pages(text: &str) -> Vec<String> {
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
