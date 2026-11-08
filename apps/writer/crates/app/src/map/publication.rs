//! Prepared graphs publish conditionally; completed personal-state writes always reconcile.
use super::*;
impl Writer {
    pub fn accept_map(
        &mut self,
        prepared: &mut Prepared,
        workspace: Workspace,
    ) -> Result<(), String> {
        if !prepared.stamp.is_current(self.session.model().document()) {
            return Err(
                "The scene changed while its map was preparing. Your current work stays open."
                    .into(),
            );
        }
        if let Some(scene) = self.map.scene(self)
            && prepared
                .positions
                .as_ref()
                .is_some_and(|positions| !Arc::ptr_eq(positions, &scene.projection.positions))
        {
            return Err("Map placement changed while routes were preparing.".into());
        }
        if self
            .map
            .scene(self)
            .and_then(|scene| scene.placement.as_ref())
            != prepared.change.as_ref()
        {
            return Err("Map placement changed while routes were preparing.".into());
        }
        let projection = prepared
            .next
            .take()
            .ok_or("This map was already installed.")?;
        if let Some(navigation) = &mut prepared.navigation
            && let Err(error) = self
                .session
                .edit(|model| model.accept_navigation(navigation))
        {
            prepared.next = Some(projection);
            return Err(crate::feedback::file(&error));
        }
        if prepared.navigation.is_some() {
            self.retire_rules();
        }
        self.reading.prepare(self.session.model());
        prepared.retired = self.map.install(
            prepared.key.clone(),
            projection,
            self.reading.selected.clone(),
            workspace,
        );
        self.status = "Map ready".into();
        self.remember_location();
        Ok(())
    }
    /// A completed personal-state write survives cancellation of its presentation.
    pub fn reconcile_map_placement(&mut self, prepared: &mut Prepared) {
        if prepared.persistence.take().is_none() {
            return;
        }
        let Some(scene) = self.map.scenes.get_mut(&prepared.key) else {
            return;
        };
        if prepared
            .positions
            .as_ref()
            .is_none_or(|positions| !Arc::ptr_eq(positions, &scene.projection.positions))
            || scene.placement.as_ref() != prepared.change.as_ref()
        {
            return;
        }
        let Some(next) = prepared.next.take() else {
            return;
        };
        scene.placement = None;
        if scene.projection.stamp.as_ref() == Some(&prepared.stamp) {
            prepared.retired = Some(std::mem::replace(&mut scene.projection, next));
            scene.reconcile_selection();
        } else {
            scene.projection.positions = next.positions.clone();
            scene.projection.placement = next.placement.clone();
            scene.projection.stamp = None;
            prepared.retired = Some(next);
        }
    }
}
