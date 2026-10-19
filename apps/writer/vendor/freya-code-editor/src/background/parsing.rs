//! Parser cache coordinates and completed-parse state, independent of mailbox policy.
use super::{Mailbox, Output, Request, language_eq, same};
use crate::{
    editor_theme::EditorSyntaxTheme,
    languages::EditorLanguage,
    syntax::{SyntaxBlocks, SyntaxHighlighter},
};
use ropey::Rope;
use std::sync::{Arc, atomic::Ordering};

#[derive(Default)]
pub(super) struct Parsing {
    highlighter: SyntaxHighlighter,
    stamp: Option<super::Stamp>,
    source: Option<Rope>,
    language: Option<EditorLanguage>,
    theme: Option<EditorSyntaxTheme>,
    parsed: bool,
}
impl Parsing {
    pub(super) fn highlight(&mut self, request: Request, mailbox: &Mailbox) -> Option<Output> {
        let changed_config = !language_eq(&self.language, &request.language)
            || self.theme.as_ref() != Some(&request.theme);
        let unchanged = self
            .source
            .as_ref()
            .is_some_and(|old| old.is_instance(&request.rope));
        let reuse = self.parsed
            && unchanged
            && !changed_config
            && same(&request.base, &self.stamp)
            && request.edits.as_ref().is_some_and(Vec::is_empty);
        if changed_config {
            self.highlighter
                .set_language(request.language.as_ref(), &request.theme);
        } else if !same(&request.base, &self.stamp) || request.edits.is_none() {
            self.highlighter.invalidate_tree();
        } else if let Some(edits) = &request.edits {
            if edits.is_empty() && !unchanged {
                self.highlighter.invalidate_tree();
            } else {
                for edit in edits {
                    self.highlighter.edit(edit);
                }
            }
        }
        // Finish a cold parse so sustained typing cannot repeatedly discard
        // all progress toward the first reusable tree.
        let incremental = self.highlighter.has_tree();
        let mut blocks = SyntaxBlocks::default();
        // The retained tree is now in this request's coordinates even when
        // parsing is interrupted. Preserve that edit chain, but never query
        // the edited old tree as if it were a completed parse.
        self.stamp = Some(Arc::clone(&request.stamp));
        self.source = Some(request.rope.clone());
        self.language = request.language;
        self.theme = Some(request.theme.clone());
        self.parsed = if reuse && self.highlighter.has_current_tree() {
            self.highlighter.highlight_rows(
                &request.rope,
                &mut blocks,
                &request.theme,
                request.rows,
            );
            true
        } else {
            self.highlighter.parse_cancellable(
                &request.rope,
                &mut blocks,
                &request.theme,
                request.rows,
                || {
                    mailbox.closed.load(Ordering::Relaxed)
                        || (incremental && mailbox.superseded.load(Ordering::Relaxed))
                },
            )
        };
        if !self.parsed {
            self.highlighter.reset_parse();
            return None;
        }
        Some(Output {
            stamp: request.stamp,
            blocks,
        })
    }
}

#[cfg(test)]
mod tests;
