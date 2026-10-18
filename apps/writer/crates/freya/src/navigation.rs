//! Freya owns history; the workspace validates locations before accepting them.
#[cfg(target_os = "linux")]
pub(crate) mod activation;
mod apply;
pub(crate) mod location;
pub(crate) mod request;
pub(crate) mod worker;
use crate::{
    AppMode,
    editing::{Pane, Writer},
    localisation::CatalogueView,
};

use freya::{prelude::*, router::*};
use location::{Location, Screen};
use recite_writer_model::View;

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

/// A matching initial route can avoid constructing a view that will never be shown.
/// Route application still owns validation, navigation and draft protection.
pub(crate) fn initial_view(
    route: Option<&str>,
    document: &str,
) -> Option<recite_config::WriterView> {
    let location = route?.parse::<Location>().ok()?;
    if location.screen != Screen::Write
        || (!location.document.is_empty() && location.document != document)
    {
        return None;
    }
    if location.source {
        Some(recite_config::WriterView::Source)
    } else {
        location.view
    }
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

pub(crate) fn snapshot(writer: Writer) -> Location {
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

fn settled_snapshot(writer: Writer) -> Option<Location> {
    if writer.work.busy() {
        return None;
    }
    let location = snapshot(writer);
    // Failed Rules reloads can also leave the screen without a reply.
    (location.screen != Screen::Rules || location.passage.is_some()).then_some(location)
}

/// Observe accepted editor transitions, including scene navigation outside the toolbar.
/// Queue edits and focus within a beat replace the current location, so typing
/// does not flood Back history.
pub(super) fn track(writer: Writer, mode: AppMode) {
    let initial = use_try_consume::<InitialRoute>();
    let startup_project = use_try_consume::<crate::InitialProject>();
    let mut pending = use_state(move || initial.map(|r| r.0));
    let mut started = use_state(|| false);
    let mut starting = use_state(|| false);
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
        if !*started.read() {
            if *starting.read() {
                return;
            }
            if let Some(link) = pending.write().take() {
                starting.set(true);
                let completed = EventHandler::new(move |result: Result<(), String>| {
                    if let Err(error) = result {
                        report_error(writer, error);
                    }
                    starting.set(false);
                    started.set(true);
                    let _ = RouterContext::get().replace(snapshot(writer));
                });
                let result = link
                    .parse::<Location>()
                    .map_err(str::to_owned)
                    .and_then(|location| route(writer, location, None, completed.clone()));
                if let Err(error) = result {
                    completed.call(Err(error));
                }
            } else {
                started.set(true);
                let _ = router.replace(snapshot(writer));
            }
        } else {
            let Some(next) = settled_snapshot(writer) else {
                return;
            };
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
    if writer.work.busy() || writer.localisation.peek().modal_open() || *writer.settings_open.peek()
    {
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
    let completed = EventHandler::new(move |result: Result<(), String>| {
        if let Err(error) = result {
            let router = RouterContext::get();
            if forward {
                router.go_back();
            } else {
                router.go_forward();
            }
            report_error(writer, error);
        }
    });
    if let Err(error) = route(writer, target, None, completed.clone()) {
        completed.call(Err(error));
    }
}

pub(super) fn open_passage(
    writer: Writer,
    document: Option<&str>,
    beat: &str,
    passage: &str,
    completed: EventHandler<Result<(), String>>,
) {
    let mut location = snapshot(writer);
    location.screen = Screen::Localise;
    if let Some(document) = document {
        location.document = document.to_owned();
    }
    location.beat = Some(beat.into());
    location.passage = Some(passage.into());
    location.source = false;
    if let Err(error) = route(writer, location, None, completed.clone()) {
        completed.call(Err(error));
    }
}

pub(super) fn route(
    writer: Writer,
    location: Location,
    cancelled: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    completed: EventHandler<Result<(), String>>,
) -> Result<(), String> {
    writer.work.require_idle()?;
    crate::projections::cancel_view(writer);
    writer.remember_scene();
    writer.buffers.harvest();
    crate::editor_work::navigate(
        writer,
        request::Request {
            target: request::Target::Location(location),
            cancelled,
        },
        completed,
    )
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
            .enabled(router.can_go_back() && !writer.work.busy()),
        )
        .child(
            crate::controls::IconButton::new(
                "Forward",
                crate::controls::Icon::Forward,
                move || step(writer, true),
            )
            .enabled(router.can_go_forward() && !writer.work.busy()),
        )
        .child(
            crate::controls::IconButton::new(
                "Copy link to this view",
                crate::controls::Icon::Link,
                move || {
                    let Some(location) = settled_snapshot(writer) else {
                        return;
                    };
                    let link = format!("recite://writer{location}");
                    writer.message.report(
                        Clipboard::set(link).map_err(|e| format!("Could not copy link: {e:?}")),
                        "Link copied.".into(),
                    );
                },
            )
            .enabled(settled_snapshot(writer).is_some()),
        )
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
