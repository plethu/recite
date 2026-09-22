//! Project declarations keep generated output and source ownership separate.
mod entries;
mod producer;
mod producer_controls;
pub(crate) use producer_controls::ProducerPoll;
mod recovery;
mod session;
use entries::entries;
mod source;
mod source_link;
use crate::{
    design::{Button, PickerOption, SearchPicker, tokens as t},
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) use session::Session;

pub(crate) fn open(mut writer: Writer) {
    if let Err(error) = try_open(writer) {
        writer.message.error(error);
    }
}
pub(crate) fn try_open(mut writer: Writer) -> Result<(), String> {
    (|| {
        let mut files = writer.files.write();
        let project = files.as_mut().ok_or("Open a project first.".to_owned())?;
        if project.declarations.is_none() {
            project.declarations = Some(Session::open(project.root()).map_err(|e| e.to_string())?);
        }
        Ok::<_, String>(())
    })()?;
    writer.pane.set(Pane::Declarations);
    Ok(())
}
#[derive(Clone, Copy)]
pub(crate) struct Declarations {
    pub writer: Writer,
}
impl PartialEq for Declarations {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for Declarations {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let query = use_state(String::new);
        let mut selected = use_state(String::new);
        let editing = use_state(|| false);
        let picker_open = use_state(|| false);
        let picker_id = use_a11y();
        let input_id = use_a11y();
        let files = writer.files.read();
        let mut body = rect()
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_LG)
                    .child(
                        label()
                            .text(text(MsgId::WriterDeclarations))
                            .font_size(t::title()),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| writer.pane.set(Pane::Map))
                            .child(text(MsgId::WriterReturnWriting)),
                    ),
            );
        let Some(session) = files.as_ref().and_then(|f| f.declarations.as_ref()) else {
            return body;
        };
        let owner = session
            .schema
            .producer_metadata
            .as_ref()
            .and_then(|m| m.producer.as_ref())
            .map_or_else(
                || text(MsgId::WriterUnknownProducer),
                |p| format!("{} · {}", p.id(), p.kind()),
            );
        body = body.child(label().text(owner)).child(
            label()
                .text(
                    if *editing.read() {
                        session.source.as_ref().map_or(&session.output, |s| &s.path)
                    } else {
                        &session.output
                    }
                    .display()
                    .to_string(),
                )
                .color(t::colors().muted),
        );
        if *editing.read() {
            return body.child(source::SourceEditor { writer, editing });
        }
        if session
            .schema
            .producer_metadata
            .as_ref()
            .and_then(|m| m.producer.as_ref())
            .is_some_and(|p| p.kind() == "standalone")
        {
            body = body.child(source::SourceActions { writer, editing });
        }
        body = body.child(producer_controls::ProducerActions { writer });
        let rows = entries(&session.schema);
        let active = rows
            .iter()
            .find(|(name, kind, _)| format!("{kind}:{name}") == selected.read().as_str())
            .or_else(|| rows.first());
        let options = std::sync::Arc::new(
            rows.iter()
                .filter(|(name, kind, _)| {
                    format!("{name} {kind}")
                        .to_lowercase()
                        .contains(&query.read().trim().to_lowercase())
                })
                .map(|(name, kind, _)| PickerOption {
                    annotation: String::new(),
                    value: format!("{kind}:{name}"),
                    title: name.clone(),
                    detail: kind.clone(),
                })
                .collect(),
        );
        let picker = SearchPicker {
            id: picker_id,
            input_id,
            name: text(MsgId::WriterDeclarations),
            placeholder: text(MsgId::WriterSearchDeclarations),
            empty_hint: text(MsgId::WriterSearchDeclarations),
            no_matches: text(MsgId::WriterNoMatches),
            selected: active
                .map(|(name, kind, _)| format!("{name} · {kind}"))
                .unwrap_or_default(),
            query,
            open: picker_open,
            options,
            choose: EventHandler::new(move |option: PickerOption| selected.set(option.value)),
            vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
            enabled: true,
        };
        let detail = if let Some((name, kind, description)) = active {
            rect()
                .width(Size::fill())
                .spacing(t::SPACE_MD)
                .child(label().text(name.clone()).font_size(t::heading()))
                .child(label().text(kind.clone()).color(t::colors().muted))
                .child(
                    paragraph()
                        .width(Size::fill())
                        .span(Span::new(description.clone())),
                )
        } else {
            rect().child(label().text(text(MsgId::WriterNoMatches)))
        };
        body.child(
            rect()
                .width(Size::fill())
                .max_width(Size::px(640.))
                .child(picker),
        )
        .child(source_link::SourceLink {
            writer,
            name: active.map(|(name, _, _)| name.clone()).unwrap_or_default(),
            kind: active.map(|(_, kind, _)| kind.clone()).unwrap_or_default(),
        })
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(detail),
        )
    }
}
