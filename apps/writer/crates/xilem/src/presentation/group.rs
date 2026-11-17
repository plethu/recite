//! Name a composed native pane without owning its scrolling, focus or product state.
#[cfg(test)]
mod tests;
mod widget;
pub(crate) use widget::Group;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{accesskit::Role, core::ArcStr, properties::Dimensions},
};

pub(crate) struct Named<V> {
    name: ArcStr,
    description: ArcStr,
    role: Role,
    level: Option<usize>,
    disabled: bool,
    child: V,
}
pub(crate) fn named<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        level: None,
        role: Role::Group,
        disabled: false,
        child,
    }
}
/// `child` with no role of its own, made inert while `disabled`: the document behind a modal
/// surface, which keeps its layout but neither takes input nor offers its controls.
pub(crate) fn inert<V>(disabled: bool, child: V) -> Named<V> {
    Named {
        name: ArcStr::from(""),
        description: ArcStr::from(""),
        level: None,
        role: Role::GenericContainer,
        disabled,
        child,
    }
}
pub(crate) fn list<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        level: None,
        role: Role::List,
        disabled: false,
        child,
    }
}
/// Tabs: one Tab stop, the selected tab, with Left and Right moving between them (see the native
/// host's `radio` arrows).
pub(crate) fn tab_list<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        level: None,
        role: Role::TabList,
        disabled: false,
        child,
    }
}
pub(crate) fn status<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        level: None,
        role: Role::Status,
        disabled: false,
        child,
    }
}
/// `child` as a named dialog while `modal`, and a plain container otherwise: a slip that becomes a
/// decision once its content is ready, without rebuilding its controls.
pub(crate) fn dialog_while<V>(modal: bool, name: impl Into<ArcStr>, child: V) -> Named<V> {
    if modal {
        dialog(name, child)
    } else {
        inert(false, child)
    }
}
pub(crate) fn dialog<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        level: None,
        role: Role::Dialog,
        disabled: false,
        child,
    }
}
/// `child`, the words of a heading, in the Heading role at `level`, named by those words, so
/// assistive tech can move between headings: 1 for a page or scene title, 2 for a beat or a
/// section, 3 for a section within one.
pub(crate) fn heading<V>(level: usize, name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        description: ArcStr::from(""),
        role: Role::Heading,
        level: Some(level),
        disabled: false,
        child,
    }
}
impl<V> Named<V> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// What the group says beyond its name: the whole of a message it shows abbreviated.
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = description.into();
        self
    }
}
impl<V> ViewMarker for Named<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Named<V> {
    type Element = Pod<Group>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        let mut pod = Pod::new_with_props(
            Group::new(self.name.clone(), self.role, child.new_widget)
                .with_description(self.description.clone())
                .with_level(self.level),
            Dimensions::MAX,
        );
        pod.new_widget.options.disabled = self.disabled;
        (pod, local)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        if self.disabled != previous.disabled {
            pod.ctx.set_disabled(self.disabled);
        }
        if self.name != previous.name {
            Group::set_name(&mut pod, self.name.clone());
        }
        if self.description != previous.description {
            Group::set_description(&mut pod, self.description.clone());
        }
        if self.role != previous.role {
            Group::set_role(&mut pod, self.role);
        }
        if self.level != previous.level {
            Group::set_level(&mut pod, self.level);
        }
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                Group::child_mut(&mut pod).downcast(),
                state,
            )
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child
                .teardown(local, ctx, Group::child_mut(&mut pod).downcast())
        });
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_first() != Some(ViewId::new(0)) {
            return MessageResult::Stale;
        }
        self.child
            .message(local, message, Group::child_mut(&mut pod).downcast(), state)
    }
}
