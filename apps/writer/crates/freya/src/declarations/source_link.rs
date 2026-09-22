use super::*;
#[derive(Clone)]
pub(super) struct SourceLink {
    pub writer: Writer,
    pub name: String,
    pub kind: String,
}
impl PartialEq for SourceLink {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for SourceLink {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let files = writer.files.read();
        let Some(project) = files.as_ref() else {
            return rect();
        };
        let Some(registration) = project
            .declarations
            .as_ref()
            .and_then(|s| s.registration.as_ref().ok())
            .and_then(|r| r.as_ref())
        else {
            return rect();
        };
        let Some(source) = registration
            .sources()
            .iter()
            .find(|s| s.name() == self.name && super::entries::kind_label(s.kind()) == self.kind)
        else {
            return rect();
        };
        let location = format!("{}:{}:{}", source.file(), source.line(), source.column());
        let mut body = rect().spacing(t::SPACE_SM).child(label().text(location));
        let Some(editor) = registration.editor().cloned() else {
            return body.child(label().text(text(MsgId::WriterConfigureSourceEditor)));
        };
        let source = source.clone();
        let root = project.root().to_owned();
        body = body.child(
            Button::new()
                .enabled(project.handoff.is_none())
                .on_press(move |_| {
                    let result = (|| {
                        let path = root
                            .join(source.file())
                            .canonicalize()
                            .map_err(|e| e.to_string())?;
                        let root = root.canonicalize().map_err(|e| e.to_string())?;
                        if !path.starts_with(&root) || !path.is_file() {
                            return Err(
                                "Declaration source must be a file inside this project.".into()
                            );
                        }
                        let command = super::producer::command(
                            &root,
                            &editor,
                            &[
                                ("{file}", path.to_string_lossy().into_owned()),
                                ("{line}", source.line().to_string()),
                                ("{column}", source.column().to_string()),
                            ],
                        )?;
                        crate::external::Handoff::command(command)
                    })();
                    match result {
                        Ok(job) => {
                            if let Some(p) = writer.files.write().as_mut() {
                                p.handoff = Some(job);
                            }
                            writer.message.info(text(MsgId::WriterExternalOpening));
                        }
                        Err(e) => writer.message.error(e),
                    }
                })
                .child(text(MsgId::WriterOpenDeclarationSource)),
        );
        body
    }
}
