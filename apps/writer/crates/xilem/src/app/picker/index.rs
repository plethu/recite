//! Labels and accepted lookup inputs are captured once, not reconstructed on every render.
use super::*;
use crate::reading::Content;
use recite_writer_model::{Passage, View};
use recite_writer_session::commands::{self, targets};
#[cfg(test)]
mod tests;

pub(super) fn entries(app: &App, search: Search) -> Result<Vec<Entry>, String> {
    let Some(writer) = &app.writer else {
        return Ok(Vec::new());
    };
    match search {
        Search::Commands => Ok(WriterCommand::ALL
            .iter()
            .map(|command| Entry {
                item: Item::Command(*command),
                title: commands::label(*command, writer.attention.is_focused()),
                detail: if *command == WriterCommand::Source {
                    "Open in your preferred editor".into()
                } else {
                    writer.config.writer.shortcuts.binding(*command).replace(
                        "Primary",
                        if cfg!(target_os = "macos") {
                            "Cmd"
                        } else {
                            "Ctrl"
                        },
                    )
                },
            })
            .collect()),
        Search::GoTo => {
            let model = writer.session.model();
            let targets = if writer.session.project_root().is_some() {
                targets::project(model)
            } else {
                targets::examples(model).map_err(|error| error.to_string())?
            };
            Ok(targets
                .into_iter()
                .map(|target| Entry {
                    title: target.label(),
                    detail: if target.beat.is_some() {
                        "Beat"
                    } else {
                        "Scene"
                    }
                    .into(),
                    item: Item::Lookup(target),
                })
                .collect())
        }
        Search::ProjectText => Ok(Vec::new()),
        Search::Passages => Ok(writer
            .reading
            .rows
            .iter()
            .filter_map(|row| {
                let Content::Passage { passage, number } = &row.content else {
                    return None;
                };
                Some(Entry {
                    title: format!("{} · {}", number.ordinal(), passage.label),
                    detail: String::new(),
                    item: Item::Passage {
                        passage: visible_passage(writer, passage),
                        location: Location {
                            document: writer.session.model().document().key().to_string(),
                            beat: Some(passage.section.clone()),
                            passage: Some(passage.id.clone()),
                            view: Some(recite_config::WriterView::Script),
                            ..Location::default()
                        },
                    },
                })
            })
            .collect()),
    }
}
fn visible_passage(writer: &Writer, passage: &Passage) -> Passage {
    let mut visible = passage.clone();
    if writer.session.model().view() == &View::Passage(passage.id.clone()) {
        visible.text = writer.session.model().draft().to_owned();
    }
    visible
}
impl Picker {
    pub(super) fn refresh(
        &mut self,
        keymap: recite_config::Keymap,
        search: Option<&recite_writer_model::SearchIndex>,
    ) {
        self.revision.advance();
        if self.mode() == Mode::ProjectText {
            let results = search
                .map(|index| index.search_results(&self.query, self.limit))
                .unwrap_or_default();
            self.total = results.total();
            self.entries = (0..results.len())
                .filter_map(|index| results.get(index))
                .map(|hit| {
                    let mut title = format!(
                        "{} · {}",
                        hit.document,
                        recite_writer_session::choices::display_name(&hit.beat)
                    );
                    if !hit.speaker.is_empty() {
                        title.push_str(&format!(" · {}", hit.speaker));
                    }
                    Entry {
                        title,
                        detail: hit.excerpt(&self.query, 120),
                        item: Item::Location(Location {
                            document: hit.document.to_string(),
                            beat: Some(hit.beat.to_string()),
                            passage: hit.passage.as_ref().map(ToString::to_string),
                            view: Some(recite_config::WriterView::Script),
                            ..Location::default()
                        }),
                    }
                })
                .collect();
            self.matches = (0..self.entries.len()).collect();
        } else {
            let alias = (self.mode() == Mode::Commands && keymap == recite_config::Keymap::Vim)
                .then(|| commands::vim_alias(&self.query))
                .flatten();
            let mut ranked = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    let rank = match alias {
                        Some(command) => {
                            matches!(entry.item, Item::Command(candidate) if candidate == command)
                                .then_some(0)
                        }
                        None => targets::rank(&entry.title, &self.query)
                            .or_else(|| match &entry.item {
                                Item::Passage { passage, .. } => {
                                    targets::rank(&passage.text, &self.query)
                                }
                                _ => None,
                            })
                            .or_else(|| match &entry.item {
                                Item::Destination(value) | Item::RenameBeat(value) => {
                                    targets::rank(value, &self.query)
                                }
                                _ => None,
                            }),
                    };
                    rank.map(|rank| (rank, index))
                })
                .collect::<Vec<_>>();
            ranked.sort_by_key(|(rank, _)| *rank);
            self.matches = ranked.into_iter().map(|(_, index)| index).collect();
            self.total = self.matches.len();
        }
        self.previews.invalidate();
        self.prepare_previews();
    }
}
