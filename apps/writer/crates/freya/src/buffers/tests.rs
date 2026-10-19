use super::*;
use freya::text_edit::TextEditor;
use freya_testing::prelude::*;

#[test]
fn same_source_navigation_keeps_rope_and_caret_but_scene_changes_reset_them() {
    fn app() -> impl IntoElement {
        let source = ":: start default\n-> END\n";
        let mut buffers = Buffers {
            bookmarks: Bookmarks::new(),
            rules: use_state(|| None),
            model: use_state(move || {
                let mut model = Workbench::new(source)?;
                model.select(View::Source)?;
                Ok(model)
            }),
            editor: use_state(move || editor_data(source, true, false)),
            prose: use_state(String::new),
        };
        use_hook(move || {
            buffers.editor.write().move_cursor_to(7);
            buffers.remember();
            let source = buffers.editor.peek().rope.clone();
            buffers.sync_navigation(&View::Source, false, false);
            assert!(buffers.editor.peek().rope.is_instance(&source));
            assert_eq!(buffers.editor.peek().cursor_pos(), 7);
            buffers.sync_navigation(&View::Source, true, false);
            assert!(!buffers.editor.peek().rope.is_instance(&source));
        });
        label().text("Navigation published")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}
impl Buffers {
    fn save_with(
        self,
        mut files: State<Option<ProjectFiles>>,
        save: impl FnOnce(
            &mut ProjectFiles,
            &mut recite_writer_model::Workbench,
        ) -> Result<(), crate::project::FileError>,
    ) -> Result<(), String> {
        self.harvest();
        let mut model = self.model;
        let mut state = model.write();
        let workbench = state.as_mut().map_err(|error| error.to_string())?;
        if workbench.view() == &View::Source {
            let request = workbench
                .edit_request(recite_writer_model::EditOperation::Apply)
                .map_err(|e| e.to_string())?
                .ok_or("Source request")?;
            let request = if let Some(rules) = self.rules.peek().as_ref() {
                request.with_rules(rules.clone())
            } else {
                request
            };
            let mut prepared = request
                .prepare(&recite_compiler::authoring::CancellationToken::new())
                .map_err(|e| e.to_string())?;
            workbench
                .accept_edit(&mut prepared)
                .map_err(|e| e.to_string())?;
        }
        let mut opened = files.write();
        let project = opened.as_mut().ok_or("Open a project before saving.")?;
        let outcome = save(project, workbench).map_err(|error| error.to_string());
        drop(opened);
        drop(state);
        outcome
    }
    pub(crate) fn save_all(self, files: State<Option<ProjectFiles>>) -> Result<(), String> {
        self.save_with(files, ProjectFiles::save_all)
    }
}
