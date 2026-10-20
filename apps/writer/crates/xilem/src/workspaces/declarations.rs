//! Bounded declaration inspection; the shared Session remains the canonical schema/source owner.
use super::*;
use crate::attention::{Pane, Region};
use crate::presentation::{label, paragraph, text_button};
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    masonry::properties::TextAreaLabel,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, text_input},
};
const PAGE: usize = 5;
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
    rows: Vec<Row>,
    query: String,
    matches: Vec<usize>,
    page: usize,
    selected: Option<usize>,
    detail_page: usize,
}
impl Panel {
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
pub(super) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations;
    flex_col((
        label(scale, "Declarations").text_size(28. * scale),
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
    .prop(Region(Some(Pane::Declarations)))
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
