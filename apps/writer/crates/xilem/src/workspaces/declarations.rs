//! Bounded declaration inspection; the shared Session remains the canonical schema/source owner.
use super::*;
mod outcome;
mod producer;
mod source;
use crate::attention::{Pane, Region};
use crate::presentation::{label, paragraph, radio_button, text_button};
use recite_writer_session::declarations::SchemaOutputStatus;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    masonry::properties::TextAreaLabel,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, radio_group, text_input},
};
const PAGE: usize = 5;
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Inspect,
    Source,
    Producer,
}
impl Writer {
    pub(crate) fn show_declaration_source(&mut self) {
        self.workspaces.declarations.mode = Mode::Source;
    }
    pub(crate) fn show_declaration_producer(&mut self) {
        self.workspaces.declarations.mode = Mode::Producer;
    }
}
pub(crate) struct Row {
    name: String,
    kind: String,
    folded: String,
    detail: Vec<String>,
}
impl Row {
    pub fn new((name, kind, detail): (String, String, String)) -> Self {
        let folded = format!("{name} {kind}").to_lowercase();
        Self {
            name,
            kind,
            folded,
            detail: super::text::pages(&detail),
        }
    }
}
#[derive(Default)]
pub(super) struct Panel {
    mode: Mode,
    producer: producer::Panel,
    pub(in crate::workspaces) source: source::Panel,
    rows: Vec<Row>,
    query: String,
    matches: Vec<usize>,
    page: usize,
    selected: Option<usize>,
    detail_page: usize,
}
impl Panel {
    pub fn describe_producer(&mut self, pages: Vec<String>) {
        self.producer = producer::Panel { pages, page: 0 };
    }
    pub fn replace(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.filter(self.query.clone());
    }
    fn filter(&mut self, query: String) {
        let needle = query.trim().to_lowercase();
        self.matches = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.folded.contains(&needle).then_some(index))
            .collect();
        self.query = query;
        self.page = 0;
        self.selected = self.matches.first().copied();
        self.detail_page = 0;
    }
    fn choose(&mut self, name: &str, kind: &str) {
        self.selected = self
            .rows
            .iter()
            .position(|row| row.name == name && row.kind == kind);
        self.detail_page = 0;
    }
}
pub(super) use producer::description as producer_description;
pub(super) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Declarations").text_size(28. * scale),
        radio_group(flex_row((
            radio_button(
                scale,
                "Inspect declarations",
                writer.workspaces.declarations.mode == Mode::Inspect,
                |writer: &mut Writer| writer.workspaces.declarations.mode = Mode::Inspect,
            ),
            radio_button(
                scale,
                "Declaration source",
                writer.workspaces.declarations.mode == Mode::Source,
                Writer::show_declaration_source,
            ),
            radio_button(
                scale,
                "Producer",
                writer.workspaces.declarations.mode == Mode::Producer,
                Writer::show_declaration_producer,
            ),
        ))),
        xilem::view::indexed_stack((
            inspection(writer),
            source::panel(writer),
            producer::controls(writer),
        ))
        .active(match writer.workspaces.declarations.mode {
            Mode::Inspect => 0,
            Mode::Source => 1,
            Mode::Producer => 2,
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Declarations)))
}
fn inspection(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    flex_col((
        writer
            .declaration_session()
            .filter(|session| session.output_status() == SchemaOutputStatus::Damaged)
            .map(|_| {
                paragraph(
                    scale,
                    "Generated schema is damaged; showing the last valid declarations.",
                    640.,
                )
            }),
        label(scale, "Search declarations"),
        text_input(panel.query.clone(), |writer: &mut Writer, query| {
            writer.workspaces.declarations.filter(query)
        })
        .placeholder("Find a declaration by name or kind")
        .text_size(16. * scale)
        .prop(TextAreaLabel::new("Search declarations")),
        label(
            scale,
            format!(
                "Showing {}–{} of {} declarations",
                if panel.matches.is_empty() {
                    0
                } else {
                    panel.page * PAGE + 1
                },
                ((panel.page + 1) * PAGE).min(panel.matches.len()),
                panel.matches.len()
            ),
        ),
        declaration_rows(writer),
        flex_row((
            text_button(scale, "Previous declarations", |writer: &mut Writer| {
                writer.workspaces.declarations.page =
                    writer.workspaces.declarations.page.saturating_sub(1);
            })
            .disabled(panel.page == 0),
            text_button(scale, "Next declarations", |writer: &mut Writer| {
                writer.workspaces.declarations.page += 1
            })
            .disabled((panel.page + 1) * PAGE >= panel.matches.len()),
        )),
        declaration_detail(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn declaration_rows(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.declarations;
    flex_col(
        panel
            .matches
            .iter()
            .skip(panel.page * PAGE)
            .take(PAGE)
            .map(|index| {
                let Row { name, kind, .. } = &panel.rows[*index];
                let (name, kind) = (name.clone(), kind.clone());
                text_button(
                    writer.ui_scale(),
                    format!(
                        "{} · {name} · {kind}",
                        if panel.selected == Some(*index) {
                            "Selected"
                        } else {
                            "Declaration"
                        }
                    ),
                    move |writer: &mut Writer| writer.workspaces.declarations.choose(&name, &kind),
                )
            })
            .collect::<Vec<_>>(),
    )
    .cross_axis_alignment(CrossAxisAlignment::Start)
}
fn declaration_detail(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.declarations;
    let Row {
        name, kind, detail, ..
    } = panel.selected.and_then(|index| panel.rows.get(index))?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(scale, "Selected declaration").text_size(18. * scale),
            label(scale, format!("{name} · {kind}")),
            paragraph(
                scale,
                detail.get(panel.detail_page).cloned().unwrap_or_default(),
                640.,
            ),
            flex_row((
                text_button(
                    scale,
                    "Previous declaration detail",
                    |writer: &mut Writer| {
                        writer.workspaces.declarations.detail_page =
                            writer.workspaces.declarations.detail_page.saturating_sub(1);
                    },
                )
                .disabled(panel.detail_page == 0),
                text_button(scale, "More declaration detail", |writer: &mut Writer| {
                    writer.workspaces.declarations.detail_page += 1
                })
                .disabled(detail.len() <= panel.detail_page + 1),
            )),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
}
