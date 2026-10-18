//! Freya owns history; the workspace validates locations before accepting them.
#[cfg(target_os = "linux")]
pub(crate) mod activation;
mod location;
mod restore;
use crate::{
    AppMode,
    editing::{Pane, Writer},
    localisation::CatalogueView,
};
use freya::{prelude::*, router::*};
use location::{Location, Screen};
use recite_writer_model::View;
use restore::apply;

/// Initial workspace link for native hosts. Project links resolve within the opened project.
#[derive(Clone)]
pub struct InitialRoute(pub String);
#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
pub(crate) struct NavigationReady(pub(crate) State<bool>);

pub(crate) fn project_from_route(route: &str) -> Result<Option<std::path::PathBuf>, String> {
    let location = route.parse::<Location>().map_err(str::to_owned)?;
    Ok(location.project.map(std::path::PathBuf::from))
}

#[derive(Clone, Copy)]
pub(crate) struct Queue {
    pub search: State<String>,
    pub attention: State<bool>,
    pub page: State<usize>,
}

pub(super) fn app(mode: AppMode) -> Element {
    let mode = use_state(move || mode);
    use_provide_context(move || ModeSwitch(mode));
    Router::<Location>::new(RouterConfig::default).into_element()
}

/// The window's mode can change at runtime: the welcome screen opens the bundled examples.
#[derive(Clone, Copy)]
pub(crate) struct ModeSwitch(pub State<AppMode>);

impl ModeSwitch {
    pub(crate) fn open_examples() {
        if let Some(mut switch) = try_consume_context::<Self>() {
            switch.0.set(AppMode::Examples);
        }
    }
}

#[derive(Clone, PartialEq)]
struct Shell;
impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        let mode = *use_consume::<ModeSwitch>().0.read();
        ModeWorkbench { mode }
    }
}

/// Keyed by mode, so switching modes builds a fresh workbench instead of reusing its state.
#[derive(Clone, Copy, PartialEq)]
struct ModeWorkbench {
    mode: AppMode,
}
impl Component for ModeWorkbench {
    fn render(&self) -> impl IntoElement {
        use_provide_context(move || self.mode);
        crate::workbench::render(self.mode)
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&(self.mode as u8))
    }
}

fn snapshot(writer: Writer) -> Location {
    let model = writer.buffers.model.read();
    let state = writer.localisation.read();
    let files = writer.files.read();
    let root = files.as_ref().map(|f| f.root());
    let mut location = Location {
        screen: match *writer.pane.read() {
            Pane::Disk => Screen::Disk,
            Pane::Rename => Screen::Rename,
            Pane::Build => Screen::Build,
            Pane::Declarations => Screen::Declarations,
            Pane::Rules => Screen::Rules,
            Pane::Preview => Screen::Preview,
            Pane::Map | Pane::Script if !state.active => Screen::Write,
            Pane::Map | Pane::Script => match state.view {
                CatalogueView::Compare => Screen::Compare,
                CatalogueView::Entry => Screen::Entry,
                CatalogueView::Updates => Screen::Updates,
                CatalogueView::Queue => Screen::Translations,
                CatalogueView::Passage => Screen::Localise,
            },
        },
        entry: (state.active && state.view == crate::localisation::CatalogueView::Entry)
            .then(|| state.entry_context.clone())
            .flatten(),
        catalogue: state.catalogue.as_ref().map(|c| {
            root.and_then(|root| c.path.strip_prefix(root).ok())
                .unwrap_or(&c.path)
                .to_string_lossy()
                .into_owned()
        }),
        project: root.map(|r| r.to_string_lossy().into_owned()),
        query: writer.queue.search.read().clone(),
        attention: *writer.queue.attention.read(),
        page: if state.view == crate::localisation::CatalogueView::Updates {
            state.update_index
        } else {
            *writer.queue.page.read()
        },
        ..Location::default()
    };
    if let Ok(model) = model.as_ref() {
        location.document = model.document().key().to_string();
        location.source = model.view() == &View::Source;
        location.view = (!location.source).then(|| *writer.layout.view.read());
        if !location.source && (*writer.pane.read() != Pane::Map || state.active) {
            location.beat = model.selected_block().ok().flatten();
            if let View::Passage(id) = model.view() {
                location.passage = Some(id.clone());
            }
        }
    }
    if location.screen == Screen::Rules {
        location.source = false;
        location.passage = writer
            .buffers
            .rules
            .read()
            .as_ref()
            .map(|r| r.passage.clone());
    }
    location
}

/// Observe accepted editor transitions, including scene navigation outside the toolbar.
/// Queue edits and focus within a beat replace the current location, so typing
/// does not flood Back history.
pub(super) fn track(writer: Writer, mode: AppMode) {
    let initial = use_try_consume::<InitialRoute>();
    let startup_project = use_try_consume::<crate::InitialProject>();
    let mut pending = use_state(move || initial.map(|r| r.0));
    let mut started = use_state(|| false);
    #[cfg(target_os = "linux")]
    use_provide_context(move || NavigationReady(started));
    use_after_side_effect(move || {
        let ready = mode != AppMode::Project
            || writer.files.read().is_some()
            || startup_project
                .as_ref()
                .is_none_or(|project| project.0.is_none());
        if !ready {
            return;
        }
        let router = RouterContext::get();
        if !*started.peek() {
            started.set(true);
            if let Some(link) = pending.write().take() {
                let result = link
                    .parse::<Location>()
                    .map_err(str::to_owned)
                    .and_then(|location| apply(writer, &location));
                if let Err(error) = result {
                    report_error(writer, error);
                }
            }
            let _ = router.replace(snapshot(writer));
        } else {
            let next = snapshot(writer);
            let current = router.current::<Location>();
            if next != current {
                if next.same_place(&current) {
                    let _ = router.replace(next);
                } else {
                    let _ = router.push(next);
                }
            }
        }
    });
}

pub(super) fn step(writer: Writer, forward: bool) {
    if writer.localisation.peek().modal_open() || *writer.settings_open.peek() {
        return;
    }
    let router = RouterContext::get();
    if (forward && !router.can_go_forward()) || (!forward && !router.can_go_back()) {
        return;
    }
    if forward {
        router.go_forward();
    } else {
        router.go_back();
    }
    let target = router.current::<Location>();
    if let Err(error) = apply(writer, &target) {
        if forward {
            router.go_back();
        } else {
            router.go_forward();
        }
        report_error(writer, error);
    }
}

pub(super) fn open_passage(
    writer: Writer,
    document: Option<&str>,
    beat: &str,
    passage: &str,
) -> bool {
    let mut location = snapshot(writer);
    location.screen = Screen::Localise;
    if let Some(document) = document {
        location.document = document.to_owned();
    }
    location.beat = Some(beat.into());
    location.passage = Some(passage.into());
    location.source = false;
    match apply(writer, &location) {
        Ok(()) => true,
        Err(error) => {
            report_error(writer, error);
            false
        }
    }
}

pub(super) fn buttons(mut writer: Writer) -> Element {
    let router = RouterContext::get();
    let _ = router.full_route_string();
    rect()
        .horizontal()
        .child(
            crate::controls::IconButton::new("Back", crate::controls::Icon::Back, move || {
                step(writer, false)
            })
            .enabled(router.can_go_back()),
        )
        .child(
            crate::controls::IconButton::new(
                "Forward",
                crate::controls::Icon::Forward,
                move || step(writer, true),
            )
            .enabled(router.can_go_forward()),
        )
        .child(crate::controls::IconButton::new(
            "Copy link to this view",
            crate::controls::Icon::Link,
            move || {
                let link = format!("recite://writer{}", snapshot(writer));
                writer.message.report(
                    Clipboard::set(link).map_err(|e| format!("Could not copy link: {e:?}")),
                    "Link copied.".into(),
                );
            },
        ))
        .into_element()
}

#[cfg(test)]
mod tests;

fn report_error(mut writer: Writer, error: String) {
    if writer.localisation.peek().dirty() {
        writer.message.error_with_action(
            error,
            crate::messages::text(crate::messages::MsgId::WriterGuiOpenUnsavedTranslations),
            EventHandler::new(move |()| crate::localisation::show_unsaved(writer)),
        );
    } else {
        writer.message.error(error);
    }
}
