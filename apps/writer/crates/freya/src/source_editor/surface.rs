//! Independent line-number gutter avoids rc.7's shared horizontal scroll bounds.
use super::EditorViewport;
use freya::{code_editor::CodeEditorData, prelude::*};

#[derive(Clone)]
pub(crate) struct EditorSurface {
    pub editor: State<CodeEditorData>,
    pub viewport: EditorViewport,
    pub id: AccessibilityId,
    pub size: f32,
    pub content: Element,
}
impl PartialEq for EditorSurface {
    fn eq(&self, other: &Self) -> bool {
        self.editor == other.editor
            && self.id == other.id
            && self.size == other.size
            && self.content == other.content
    }
}
impl Component for EditorSurface {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        let mut editor = self.editor;
        use_side_effect_with_deps(&self.size, move |size| {
            editor.write().measure(*size, "monospace")
        });
        self.viewport.observe(self.editor, self.size);
        rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .height(Size::fill())
            .on_pointer_down(move |_| id.request_focus())
            .child(Gutter {
                editor: self.editor,
                scroll: self.viewport.scroll,
                size: self.size,
            })
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .child(self.content.clone()),
            )
    }
}
#[derive(Clone, PartialEq)]
struct Gutter {
    editor: State<CodeEditorData>,
    scroll: ScrollController,
    size: f32,
}
impl Component for Gutter {
    fn render(&self) -> impl IntoElement {
        let colors = crate::design::tokens::colors();
        let mut height = use_state(|| 0.);
        let (_, y): (i32, i32) = self.scroll.into();
        let line = (self.size * crate::design::tokens::CODE_LINE_HEIGHT).floor();
        let count = self.editor.read().rope.len_lines();
        let first = (-(y as f32) / line).floor().max(0.) as usize;
        let last = (first + (*height.read() / line).ceil() as usize + 1).min(count);
        rect()
            .width(Size::px(self.size * 5.))
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .on_sized(move |event: Event<SizedEventData>| {
                height.set_if_modified(event.area.height())
            })
            .children(
                (first..last)
                    .map(|index| {
                        rect()
                            .position(Position::new_absolute().top(index as f32 * line + y as f32))
                            .width(Size::fill())
                            .height(Size::px(line))
                            .main_align(Alignment::Center)
                            .cross_align(Alignment::End)
                            .padding((0., 8.))
                            .child(
                                label()
                                    .text((index + 1).to_string())
                                    .font_size(self.size)
                                    .font_family("monospace")
                                    .color(colors.muted),
                            )
                            .into_element()
                    })
                    .collect::<Vec<_>>(),
            )
    }
}
