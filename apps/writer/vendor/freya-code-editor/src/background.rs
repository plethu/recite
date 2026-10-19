//! One worker, one replaceable request, and one replaceable result per editor.
mod parsing;
use crate::{editor_theme::EditorSyntaxTheme, languages::EditorLanguage, syntax::SyntaxBlocks};
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
struct Active {
    rope: Rope,
    language: Option<EditorLanguage>,
}
#[derive(Default)]
struct Slots {
    active: Option<Active>,
    pending: Option<Request>,
    ready: Option<Output>,
}
#[derive(Default)]
struct Mailbox {
    slots: Mutex<Slots>,
    wake: Condvar,
    closed: AtomicBool,
    superseded: AtomicBool,
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
        if slots.active.as_ref().is_some_and(|active| {
            !active.rope.is_instance(&request.rope)
                || !language_eq(&active.language, &request.language)
        }) {
            self.mailbox.superseded.store(true, Ordering::Relaxed);
        }
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
        (Some(a), Some(b)) => {
            a.language == b.language
                && a.highlights_query == b.highlights_query
                && a.line_isolated == b.line_isolated
        }
        (None, None) => true,
        _ => false,
    }
}
fn run(mailbox: &Mailbox, notify: async_channel::Sender<()>) {
    let mut syntax = parsing::Parsing::default();
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
            let Some(request) = slots.pending.take() else {
                continue;
            };
            mailbox.superseded.store(false, Ordering::Relaxed);
            let obsolete = slots.active.replace(Active {
                rope: request.rope.clone(),
                language: request.language.clone(),
            });
            drop(slots);
            drop(obsolete);
            request
        };
        let Some(output) = syntax.highlight(request, mailbox) else {
            continue;
        };
        let mut slots = mailbox
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let obsolete = slots.ready.replace(output);
        drop(slots);
        drop(obsolete);
        // One queued wake already covers the latest replaceable output.
        let _ = notify.try_send(());
    }
}

#[cfg(test)]
mod tests;
