//! Completion retains Source input ownership and compiler-provided replacement spans.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::{prelude::*, text_edit::TextEditor};

#[derive(Clone)]
pub(super) struct Completion {
    pub writer: Writer,
    pub editor_id: AccessibilityId,
    pub viewport: State<Option<Area>>,
}
impl PartialEq for Completion {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
mod cache;
mod input;
use input::apply;
pub(crate) use input::keyboard;
mod request;
mod task;
mod worker;
pub(crate) use cache::SourceCompletionCache;
use cache::candidates;

impl Component for Completion {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let scroll = use_scroll_controller(ScrollConfig::default);
        let writer = self.writer;
        let editor_id = self.editor_id;
        task::use_completion_work(writer);
        let mut state = writer.source_viewport;
        use_after_side_effect(move || {
            let opened = *state.completion.read();
            if opened && *Platform::get().focused_accessibility_id.read() != editor_id {
                editor_id.request_focus();
            }
        });
        let items = use_memo(move || {
            if *state.completion.read() {
                candidates(writer)
            } else {
                cache::clear(writer);
                None
            }
        });
        crate::design::use_list_reveal(None, state.candidate, scroll, t::picker_row_height(), 4);
        let mut root = rect().child(
            Button::new()
                .flat()
                .named(text(MsgId::WriterCompleteSource))
                .on_press(move |_| {
                    cache::clear(writer);
                    state.completion.set(true);
                    state.candidate.set(None);
                    editor_id.request_focus();
                })
                .child(text(MsgId::WriterGuiCompleteCtrlSpace)),
        );
        if !*state.completion.read() {
            return root.into_element();
        }
        let Some(area) = *self.viewport.read() else {
            return root.into_element();
        };
        let editor = writer.buffers.editor.read();
        let size = t::code_size();
        let caret = crate::source_editor::caret_rect(&editor, size);
        let (x, y): (i32, i32) = state.scroll.into();
        let width = (t::PICKER_MAX_WIDTH * t::ui_scale()).min(area.width());
        let line = (size * t::CODE_LINE_HEIGHT).floor();
        let left = (area.min_x() + size * 5. + caret.left + x as f32)
            .clamp(area.min_x(), (area.max_x() - width).max(area.min_x()));
        let bottom = area.min_y() + (editor.cursor_row() + 1) as f32 * line + y as f32;
        let count = items
            .read()
            .as_ref()
            .map_or(0, |i| i.items.candidates().len());
        let height = (count.clamp(1, 6) as f32 * t::picker_row_height()).min(area.height());
        let top = if bottom + height <= area.max_y() {
            bottom
        } else {
            (bottom - line - height).max(area.min_y())
        };
        drop(editor);
        let mut popup = rect()
            .position(Position::new_global().left(left).top(top))
            .width(Size::px(width))
            .height(Size::px(height))
            .layer(Layer::Overlay)
            .a11y_role(AccessibilityRole::ListBox)
            .a11y_alt(text(MsgId::WriterCompleteSource))
            .background(colors.inset)
            .corner_radius(t::RADIUS)
            .border(Border::new().width(1.).fill(colors.boundary));
        if let Some(items) = items.read().as_ref()
            && count > 0
        {
            let entries = items.clone();
            let active = *state.candidate.read();
            popup = popup.child(
                VirtualScrollView::new_with_data(entries, move |row, entries| {
                    let entries = entries.clone();
                    let name = entries.items.candidates()[row.index].name().to_owned();
                    Button::new()
                        .flat()
                        .row_gutter()
                        .width(Size::fill())
                        .named(name.clone())
                        .option(active == Some(row.index))
                        .on_press(move |_| {
                            let mut writer = writer;
                            if let Err(error) = apply(writer, &entries, row.index) {
                                writer.message.error(error);
                            }
                            state.completion.set(false);
                            editor_id.request_focus();
                        })
                        .child(label().width(Size::fill()).text(name))
                        .into_element()
                })
                .length(count)
                .item_size(t::picker_row_height())
                .height(Size::fill())
                .scroll_controller(scroll),
            );
        }
        if count == 0 {
            popup = popup.child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .padding((0., t::SPACE_SM))
                    .main_align(Alignment::Center)
                    .child(
                        label().text(text(
                            if writer
                                .source_viewport
                                .cache
                                .read()
                                .as_ref()
                                .is_some_and(SourceCompletionCache::pending)
                            {
                                MsgId::WriterFindingCompletions
                            } else if items.read().is_some() {
                                MsgId::WriterNoCompletions
                            } else {
                                MsgId::WriterCompletionsUnavailable
                            },
                        )),
                    ),
            );
        }
        root = root.child(popup);
        root.into_element()
    }
}
