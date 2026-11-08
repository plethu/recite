//! Incoming prompts retain source order; only the selected context page is mounted.
use recite_writer_model::{SceneLink, Workbench, scene_links};
#[derive(Default)]
pub struct Incoming {
    pub links: Vec<SceneLink>,
    pub expanded: bool,
    pub page: usize,
}
impl Incoming {
    pub fn prepare(model: &Workbench, beat: &str) -> Self {
        let links = model
            .document()
            .cached_script()
            .map(|script| {
                scene_links(&script)
                    .into_iter()
                    .filter(|link| link.destination == beat && link.origin != beat)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            links,
            ..Self::default()
        }
    }
}
