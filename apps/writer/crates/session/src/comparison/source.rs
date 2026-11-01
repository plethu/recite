//! Line alignment is shared by source conflicts and rename review.
use super::ComparisonRow;
use similar::{ChangeTag, TextDiff};
impl ComparisonRow {
    pub fn source(caption: &str, before: &str, after: &str) -> Vec<Self> {
        let diff = TextDiff::configure()
            .timeout(std::time::Duration::from_millis(100))
            .diff_lines(before, after);
        let mut rows = Vec::new();
        let mut old = String::new();
        let mut new = String::new();
        let mut equal = None;
        let mut line = 1;
        let mut start = 1;
        for change in diff.iter_all_changes() {
            let next_equal = change.tag() == ChangeTag::Equal;
            if equal.is_some_and(|equal| equal != next_equal) {
                rows.push(Self {
                    caption: if caption.is_empty() {
                        format!(
                            "{} {start}",
                            crate::messages::text(crate::messages::MsgId::WriterLine)
                        )
                    } else {
                        format!("{caption}:{start}")
                    },
                    before: Some(std::mem::take(&mut old)),
                    after: Some(std::mem::take(&mut new)),
                });
                start = line;
            }
            equal = Some(next_equal);
            if change.tag() != ChangeTag::Insert {
                old.push_str(change.value());
                line += 1;
            }
            if change.tag() != ChangeTag::Delete {
                new.push_str(change.value());
            }
        }
        if equal.is_some() {
            rows.push(Self {
                caption: if caption.is_empty() {
                    format!(
                        "{} {start}",
                        crate::messages::text(crate::messages::MsgId::WriterLine)
                    )
                } else {
                    format!("{caption}:{start}")
                },
                before: Some(old),
                after: Some(new),
            });
        }
        rows
    }
}
#[cfg(test)]
mod tests;
