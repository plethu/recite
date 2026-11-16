//! A beat's script flattened into rows: passages numbered in order, group headings, notes and
//! continuations, each with the groups it sits in and its reply number.
use recite_writer_model::{Passage, PassageKind, ScriptEntry};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct PassageNumber(usize);
impl PassageNumber {
    pub fn ordinal(self) -> usize {
        self.0
    }
}

pub enum Content {
    Passage {
        passage: Passage,
        number: PassageNumber,
    },
    Heading(String),
    Note(String),
    Continuation(String),
}

pub struct Row {
    pub content: Content,
    pub groups: Arc<[String]>,
    pub reply: Option<usize>,
}

pub(crate) fn flatten(entries: &[ScriptEntry]) -> Arc<[Row]> {
    let mut rows = Vec::new();
    let mut replies = 0;
    let mut passages = 0;
    let mut pending = vec![(entries.iter(), Arc::<[String]>::from([]))];
    while let Some((items, groups)) = pending.last_mut() {
        let Some(entry) = items.next() else {
            pending.pop();
            continue;
        };
        let groups = groups.clone();
        let content = match entry {
            ScriptEntry::Passage(passage) => {
                passages += 1;
                Content::Passage {
                    passage: passage.clone(),
                    number: PassageNumber(passages),
                }
            }
            ScriptEntry::Group { heading, entries } => {
                let mut nested = groups.to_vec();
                nested.push(heading.clone());
                pending.push((entries.iter(), nested.into()));
                Content::Heading(heading.clone())
            }
            ScriptEntry::Jump(target) => Content::Continuation(target.clone()),
            ScriptEntry::Effect(text) => Content::Note(format!("Effect · {text}")),
            ScriptEntry::Source(text) => {
                Content::Note(format!("Source fragment (read only) · {text}"))
            }
        };
        let reply = if matches!(&content, Content::Passage { passage, .. } if matches!(passage.kind, PassageKind::Choice { .. }))
        {
            replies += 1;
            Some(replies)
        } else {
            None
        };
        rows.push(Row {
            content,
            groups,
            reply,
        });
    }
    rows.into()
}
