//! Report-local source excerpts. Read each requested file once and retain only requested lines.
use std::collections::{BTreeMap, BTreeSet};

use recite_core::{SourceSpan, source_lines};

enum File {
    Requested(BTreeSet<u32>),
    Loaded(BTreeMap<u32, String>),
}

pub(super) struct Excerpts<'a> {
    files: BTreeMap<&'a str, File>,
}

impl<'a> Excerpts<'a> {
    pub(super) fn new(spans: impl Iterator<Item = &'a SourceSpan>) -> Self {
        let mut files = BTreeMap::new();
        for span in spans {
            let file = files
                .entry(span.file.as_str())
                .or_insert_with(|| File::Requested(BTreeSet::new()));
            if let File::Requested(lines) = file {
                lines.insert(span.start.line());
            }
        }
        Self { files }
    }

    pub(super) fn line(&mut self, span: &SourceSpan) -> Option<&str> {
        self.load(span, |file| std::fs::read_to_string(file).ok())
    }

    fn load(
        &mut self,
        span: &SourceSpan,
        read: impl FnOnce(&str) -> Option<String>,
    ) -> Option<&str> {
        let file = self.files.get_mut(span.file.as_str())?;
        if let File::Requested(requested) = file {
            let mut lines = BTreeMap::new();
            if let Some(source) = read(&span.file) {
                let last = requested.last().copied().unwrap_or(0);
                let mut wanted = requested.iter().copied();
                let mut next = wanted.next();
                for (number, (text, _)) in (1..=last).zip(source_lines(&source)) {
                    if next == Some(number) {
                        lines.insert(number, text.to_owned());
                        next = wanted.next();
                    }
                }
            }
            *file = File::Loaded(lines);
        }
        match file {
            File::Loaded(lines) => lines.get(&span.start.line()).map(String::as_str),
            File::Requested(_) => unreachable!("file loaded above"),
        }
    }
}

#[cfg(test)]
mod tests;
