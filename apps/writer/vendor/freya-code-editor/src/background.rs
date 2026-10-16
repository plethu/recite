//! One worker, one replaceable request, and one replaceable result per editor.
use crate::{
    editor_theme::EditorSyntaxTheme,
    languages::EditorLanguage,
    syntax::{SyntaxBlocks, SyntaxHighlighter},
};
use ropey::Rope;
use std::{
    io,
    ops::Range,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};
use tree_sitter::InputEdit;

pub(crate) const MAX_EDITS: usize = 64;
type Stamp = Arc<()>;

pub(crate) struct Request {
    pub base: Option<Stamp>,
    pub stamp: Stamp,
    pub rope: Rope,
    pub edits: Option<Vec<InputEdit>>,
    pub rows: Range<usize>,
    pub language: Option<EditorLanguage>,
    pub theme: EditorSyntaxTheme,
}

pub(crate) struct Output {
    pub stamp: Stamp,
    pub blocks: SyntaxBlocks,
}
#[derive(Default)]
struct Slots {
    pending: Option<Request>,
    ready: Option<Output>,
}
#[derive(Default)]
struct Mailbox {
    slots: Mutex<Slots>,
    wake: Condvar,
    closed: AtomicBool,
}

pub(crate) struct BackgroundSyntax {
    mailbox: Arc<Mailbox>,
    worker: JoinHandle<()>,
    ready: async_channel::Receiver<()>,
}
impl BackgroundSyntax {
    pub(crate) fn new() -> io::Result<Self> {
        let mailbox = Arc::new(Mailbox::default());
        let shared = Arc::clone(&mailbox);
        let (notify, ready) = async_channel::bounded(1);
        let worker = thread::Builder::new()
            .name("editor-syntax".into())
            .spawn(move || run(&shared, notify))?;
        Ok(Self {
            mailbox,
            worker,
            ready,
        })
    }

    pub(crate) fn notifications(&self) -> async_channel::Receiver<()> {
        self.ready.clone()
    }

    pub(crate) fn submit(&self, mut request: Request) -> bool {
        if self.worker.is_finished() {
            return false;
        }
        let mut slots = self
            .mailbox
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut obsolete = slots.pending.take();
        if let Some(old) = obsolete.as_mut() {
            let continuous = same(&request.base, &Some(Arc::clone(&old.stamp)));
            request.edits = match (old.edits.take(), request.edits.take()) {
                (Some(mut before), Some(after))
                    if continuous && before.len() + after.len() <= MAX_EDITS =>
                {
                    before.extend(after);
                    Some(before)
                }
                _ => None,
            };
            request.base = old.base.take();
        }
        slots.pending = Some(request);
        drop(slots);
        drop(obsolete);
        self.mailbox.wake.notify_one();
        true
    }

    pub(crate) fn take(&self) -> Option<Result<Output, ()>> {
        let mut slots = self
            .mailbox
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(output) = slots.ready.take() {
            Some(Ok(output))
        } else if self.worker.is_finished() {
            Some(Err(()))
        } else {
            None
        }
    }
    pub(crate) fn close(&self) {
        let slots = self
            .mailbox
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.mailbox.closed.store(true, Ordering::Relaxed);
        drop(slots);
        self.mailbox.wake.notify_one();
    }
}
impl Drop for BackgroundSyntax {
    fn drop(&mut self) {
        self.close();
    }
}
fn same(a: &Option<Stamp>, b: &Option<Stamp>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
fn language_eq(a: &Option<EditorLanguage>, b: &Option<EditorLanguage>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.language == b.language && a.highlights_query == b.highlights_query,
        (None, None) => true,
        _ => false,
    }
}
fn run(mailbox: &Mailbox, notify: async_channel::Sender<()>) {
    let mut highlighter = SyntaxHighlighter::new();
    let mut stamp = None;
    let mut source: Option<Rope> = None;
    let mut language = None;
    let mut theme = None;
    loop {
        let request = {
            let mut slots = mailbox
                .slots
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while slots.pending.is_none() && !mailbox.closed.load(Ordering::Relaxed) {
                slots = mailbox
                    .wake
                    .wait(slots)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            if mailbox.closed.load(Ordering::Relaxed) {
                break;
            }
            slots.pending.take()
        };
        let Some(request) = request else {
            continue;
        };
        let changed_config =
            !language_eq(&language, &request.language) || theme.as_ref() != Some(&request.theme);
        let unchanged = source
            .as_ref()
            .is_some_and(|old| old.is_instance(&request.rope));
        if changed_config {
            highlighter.set_language(request.language.as_ref(), &request.theme);
        } else if !same(&request.base, &stamp) || request.edits.is_none() {
            highlighter.invalidate_tree();
        } else if let Some(edits) = &request.edits {
            if edits.is_empty() && !unchanged {
                highlighter.invalidate_tree();
            } else {
                for edit in edits {
                    highlighter.edit(edit);
                }
            }
        }
        let mut blocks = SyntaxBlocks::default();
        if unchanged
            && !changed_config
            && same(&request.base, &stamp)
            && request.edits.as_ref().is_some_and(Vec::is_empty)
        {
            highlighter.highlight_rows(&request.rope, &mut blocks, &request.theme, request.rows);
        } else if !highlighter.parse_cancellable(
            &request.rope,
            &mut blocks,
            &request.theme,
            request.rows,
            Some(&mailbox.closed),
        ) {
            break;
        }
        stamp = Some(Arc::clone(&request.stamp));
        source = Some(request.rope);
        language = request.language;
        theme = Some(request.theme);
        let mut slots = mailbox
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let obsolete = slots.ready.replace(Output {
            stamp: request.stamp,
            blocks,
        });
        drop(slots);
        drop(obsolete);
        // One queued wake already covers the latest replaceable output.
        let _ = notify.try_send(());
    }
}

#[cfg(test)]
mod tests;
