//! Presentation state is independent of operation results.
use crate::design::{Button, tokens as t};
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Information,
    Error,
}
#[derive(Clone, PartialEq)]
struct Notice {
    kind: Kind,
    text: String,
    action: Option<(String, EventHandler<()>)>,
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Feedback {
    notice: State<Option<Notice>>,
    action_id: AccessibilityId,
    dismiss_id: AccessibilityId,
}
impl Feedback {
    pub fn new() -> Self {
        Self {
            notice: use_state(|| None),
            action_id: use_a11y(),
            dismiss_id: use_a11y(),
        }
    }
    pub fn info(&mut self, text: String) {
        self.notice.set((!text.is_empty()).then_some(Notice {
            kind: Kind::Information,
            text,
            action: None,
        }));
    }
    pub fn error(&mut self, text: String) {
        self.notice.set(Some(Notice {
            kind: Kind::Error,
            text,
            action: None,
        }));
    }
    pub fn error_with_action(&mut self, text: String, caption: String, action: EventHandler<()>) {
        self.notice.set(Some(Notice {
            kind: Kind::Error,
            text,
            action: Some((caption, action)),
        }));
    }
    pub fn clear(&mut self) {
        self.notice.set(None);
    }
    pub fn is_empty(self) -> bool {
        self.notice.read().is_none()
    }
    pub fn is_error(self) -> bool {
        self.notice
            .read()
            .as_ref()
            .is_some_and(|notice| notice.kind == Kind::Error)
    }
    pub fn text(self) -> String {
        self.notice
            .read()
            .as_ref()
            .map(|notice| notice.text.clone())
            .unwrap_or_default()
    }
    pub fn focus_order(self) -> Vec<AccessibilityId> {
        let notice = self.notice.read();
        let Some(notice) = notice.as_ref() else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        if notice.action.is_some() {
            ids.push(self.action_id);
        }
        ids.push(self.dismiss_id);
        ids
    }
    pub fn report(&mut self, result: Result<(), String>, success: String) {
        match result {
            Ok(()) => self.info(success),
            Err(error) => self.error(error),
        }
    }
}
#[derive(Clone, PartialEq)]
pub(crate) struct NoticeView {
    pub feedback: Feedback,
}
impl Component for NoticeView {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let mut feedback = self.feedback;
        let Some(notice) = feedback.notice.read().clone() else {
            return rect().into_element();
        };
        let mut row = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_SM)
            .padding(t::SPACE_SM)
            .a11y_role(if notice.kind == Kind::Error {
                AccessibilityRole::Alert
            } else {
                AccessibilityRole::Status
            })
            .child(label().width(Size::flex(1.)).text(notice.text));
        if let Some((caption, action)) = notice.action {
            row = row.child(
                Button::new()
                    .a11y_id(feedback.action_id)
                    .named(caption.clone())
                    .on_press(move |_| action.call(()))
                    .child(caption),
            );
        }
        row.child(
            Button::new()
                .flat()
                .a11y_id(feedback.dismiss_id)
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiDismissMessage,
                ))
                .on_press(move |_| feedback.clear())
                .child("×"),
        )
        .border(Border::new().width(1.).fill(colors.rule))
        .into_element()
    }
}

#[cfg(test)]
mod tests;
