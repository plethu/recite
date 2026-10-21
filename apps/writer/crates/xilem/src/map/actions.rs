//! Checked transitions between map presentation and the shared authoring session.
use super::*;
use tokio::sync::mpsc as async_mpsc;
use xilem::masonry::kurbo;
impl Writer {
    pub(crate) fn invalidate_map(&mut self) {
        if self.map.workspace.has_map() {
            self.map.refresh = Some(self.session.model().document().key().clone());
        }
    }
    pub(crate) fn refresh_map_if_needed(&mut self) {
        if self.navigation.busy()
            || self.workspaces.screen != crate::workspaces::Screen::Write
            || self.attention.is_focused()
            || self.work.busy()
            || self.phase != crate::Phase::Writing
            || self
                .navigation
                .sender
                .as_ref()
                .is_none_or(async_mpsc::UnboundedSender::is_closed)
            || self.navigation.runtime.is_none()
        {
            return;
        }
        let Some(key) = self.map.refresh.take() else {
            return;
        };
        if self.map.workspace.has_map()
            && self.session.model().document().key() == &key
            && self
                .map
                .scene(self)
                .is_none_or(|scene| !scene.projection.is_current(self.session.model().document()))
        {
            self.refresh_map_layout();
        }
    }
    pub(crate) fn show_workspace(&mut self, workspace: Workspace) {
        self.cancel_project_workspace_open();
        self.workspaces.screen = crate::workspaces::Screen::Write;
        self.attention.leave_focus();
        self.trial.visible = false;
        self.trial.settings.open = false;
        if workspace.has_map()
            && workspace == self.map.workspace
            && self
                .map
                .scene(self)
                .is_some_and(|scene| scene.projection.is_current(self.session.model().document()))
        {
            return;
        }
        if workspace == Workspace::Script {
            self.cancel_map_gesture();
            self.cancel_map_preparation();
            self.map.workspace = workspace;
            for scene in self.map.scenes.values_mut() {
                scene.focus = None;
            }
            self.remember_location();
            return;
        }
        self.navigate(Target::Workspace(workspace));
    }
    pub(crate) fn prepare_map(&self, purpose: preparation::Purpose) -> Preparation {
        let model = self.session.model();
        let previous = self.map.scene(self);
        let scene = self.session.project().map_or_else(
            || format!("example:{}", model.document().key().as_str()),
            |project| format!("file:{:?}", project.current().path()),
        );
        Preparation {
            purpose,
            change: previous.and_then(|scene| scene.placement.clone()),
            persist: previous.is_some_and(|scene| {
                scene.projection.placement.needs_save() || scene.placement.is_some()
            }),
            status: previous
                .map(|scene| scene.projection.placement.clone())
                .unwrap_or_default(),
            model: model.navigation_request(),
            key: model.document().key().clone(),
            stamp: model.document().analysis_stamp(),
            scene,
            store: self.map.store.clone(),
            positions: previous
                .filter(|scene| {
                    !matches!(scene.projection.placement, placement::Status::LoadFailed(_))
                        || scene.placement.is_some()
                })
                .map(|scene| scene.projection.positions.clone()),
            previous: previous.map(|scene| scene.projection.graph.clone()),
        }
    }
    pub(super) fn refresh_map_layout(&mut self) {
        self.navigate(Target::MapRefresh);
    }
    pub(super) fn map_scene_mut(&mut self) -> Option<&mut Scene> {
        self.map
            .scenes
            .get_mut(self.session.model().document().key())
    }
    pub(crate) fn map_selected(&self) -> Option<&str> {
        self.map
            .scene(self)
            .and_then(|scene| scene.selected.as_deref())
    }
    pub(crate) fn sync_map_selection(&mut self) {
        let selected = self.reading.selected.clone();
        let current = self
            .map
            .scene(self)
            .is_some_and(|scene| scene.projection.is_current(self.session.model().document()));
        if let Some(scene) = self.map_scene_mut() {
            if current {
                scene.select(selected);
            } else {
                scene.selected = selected;
                scene.focus = None;
            }
        }
    }
    pub(super) fn select_map_beat(&mut self, beat: String) {
        if let Some(scene) = self.map_scene_mut() {
            scene.select(Some(beat));
        }
    }
    pub(super) fn focus_map_beat(&mut self, beat: String) {
        if let Some(scene) = self.map_scene_mut()
            && scene.projection.graph.ids.contains_key(&beat)
        {
            if scene.detail() == Detail::Overview {
                scene.camera.zoom_to(1., scene.viewport);
            }
            scene.select(Some(beat.clone()));
            scene.reveal_selected();
            scene.focus = Some(FocusIntent {
                beat,
                token: input::FocusToken::new(),
            });
        }
    }
    pub(super) fn press_map_beat(
        &mut self,
        beat: String,
        pointer: Option<xilem::view::PointerButton>,
    ) {
        if pointer.is_some_and(|button| button != xilem::view::PointerButton::Primary) {
            return;
        }
        self.select_map_beat(beat.clone());
        if pointer.is_none() {
            self.open_map_beat();
        }
    }
    pub(super) fn open_map_beat(&mut self) {
        if let Some(beat) = self
            .map
            .scene(self)
            .and_then(|scene| scene.selected.clone())
        {
            self.inspect(beat);
            self.map.workspace = Workspace::Split;
        }
    }
    pub(super) fn resize_map(&mut self, size: kurbo::Size) {
        let scale = self.ui_scale();
        if let Some(scene) = self.map_scene_mut() {
            scene.viewport = (size.width as f32 / scale, size.height as f32 / scale);
            scene.frame();
        }
    }
    pub(super) fn zoom_map(&mut self, factor: f32) {
        self.cancel_map_gesture();
        if let Some(scene) = self.map_scene_mut() {
            scene
                .camera
                .zoom_to(scene.camera.zoom * factor, scene.viewport);
        }
    }
    pub(super) fn reset_map_zoom(&mut self) {
        self.cancel_map_gesture();
        if let Some(scene) = self.map_scene_mut() {
            scene.camera.zoom_to(1., scene.viewport);
        }
    }
    pub(super) fn arrange_map(&mut self) {
        if let Some(scene) = self.map_scene_mut() {
            scene.placement = Some(placement::Change::Reset);
            scene.camera.framing = Framing::Free;
            self.refresh_map_layout();
        }
    }
    pub(super) fn fit_map(&mut self) {
        self.cancel_map_gesture();
        if let Some(scene) = self.map_scene_mut() {
            scene.camera.fit(scene.viewport, scene.bounds());
        }
    }
}
