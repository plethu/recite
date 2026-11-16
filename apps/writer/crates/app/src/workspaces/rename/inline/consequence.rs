//! What a reviewed inline rename changes, counted from the review's before and after files.
use super::*;

/// What a reviewed rename changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consequence {
    /// Changed source lines, the beat's own heading among them.
    pub references: usize,
    /// Changed files, the project manifest among them when its entry points move.
    pub files: usize,
}
impl Consequence {
    /// "Renames relay_desk to dock in 3 references across 2 files".
    pub fn sentence(self, beat: &str, name: &str) -> String {
        let plural = |count: usize, one: &str, many: &str| {
            format!("{count} {}", if count == 1 { one } else { many })
        };
        format!(
            "Renames {beat} to {name} in {} across {}",
            plural(self.references, "reference", "references"),
            plural(self.files, "file", "files"),
        )
    }
}

impl Writer {
    /// What the reviewed rename changes, while its slip is showing.
    pub fn rename_consequence(&self) -> Option<Consequence> {
        let review = self.session.project()?.files().rename_review.as_ref()?;
        let changed = review
            .plan
            .changes
            .iter()
            .map(|change| changed_lines(&change.before, &change.after))
            .chain(std::iter::once(changed_lines(
                &review.manifest_before,
                &review.manifest_after,
            )))
            .filter(|lines| *lines > 0)
            .collect::<Vec<_>>();
        Some(Consequence {
            references: changed.iter().sum(),
            files: changed.len(),
        })
    }
}

/// Lines that differ between two versions of a file a rename rewrites in place.
fn changed_lines(before: &str, after: &str) -> usize {
    let (mut old, mut new) = (before.lines(), after.lines());
    let mut count = 0;
    loop {
        match (old.next(), new.next()) {
            (None, None) => return count,
            (left, right) => count += usize::from(left != right),
        }
    }
}
