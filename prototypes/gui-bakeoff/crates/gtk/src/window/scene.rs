use super::*;

impl Window {
    /// Keep the active buffer alive while moving it into its place in the script.
    pub(super) fn refresh_scene(self: &Rc<Self>) {
        if self.field.parent().is_some() {
            self.scene.remove(&self.field);
        }
        clear(&self.scene);
        let model = self.model.borrow();
        if matches!(model.view(), View::Source) {
            self.scene.append(&self.field);
            return;
        }
        let passages = match model.document().passages() {
            Ok(passages) => passages,
            Err(_) => {
                self.scene.append(&self.field);
                return;
            }
        };
        let mut section = String::new();
        for passage in passages {
            if passage.section != section {
                section = passage.section.clone();
                let heading = gtk::Label::new(Some(&section.replace('_', " ")));
                heading.add_css_class("heading");
                heading.set_xalign(0.);
                self.scene.append(&heading);
            }
            if model.view() == &View::Passage(passage.id.clone()) {
                self.scene.append(&self.field);
                continue;
            }
            let card = column();
            card.add_css_class("passage");
            let caption = match &passage.kind {
                PassageKind::Dialogue { speaker } => {
                    speaker.as_deref().unwrap_or("Narration").replace('_', " ")
                }
                PassageKind::Choice { .. } => {
                    card.add_css_class("choice");
                    "Player choice".to_owned()
                }
            };
            let speaker = gtk::Label::new(Some(&caption));
            speaker.set_xalign(0.);
            card.append(&speaker);
            let prose = gtk::Label::new(Some(&passage.text));
            prose.add_css_class("prose");
            prose.set_xalign(0.);
            prose.set_wrap(true);
            prose.set_selectable(true);
            prose.set_max_width_chars(65);
            card.append(&prose);
            if let PassageKind::Choice { destination } = &passage.kind {
                let target = gtk::Label::new(Some(&format!(
                    "Continue to {}",
                    destination
                        .as_deref()
                        .unwrap_or("next passage")
                        .replace('_', " ")
                )));
                target.set_xalign(0.);
                card.append(&target);
            }
            let id = passage.id;
            let edit = self.button(
                &format!("Edit {caption}"),
                Box::new(move |m| m.select(View::Passage(id.clone()))),
            );
            edit.set_halign(gtk::Align::Start);
            card.append(&edit);
            self.scene.append(&card);
        }
    }
}
