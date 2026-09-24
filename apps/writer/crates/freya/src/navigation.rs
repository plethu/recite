//! Freya owns history; the workspace validates locations before accepting them.
#[cfg(target_os = "linux")]
pub(crate) mod activation;
mod location;
use crate::{
    AppMode,
    editing::{Pane, Writer},
};
use freya::{prelude::*, router::*};
use location::{Location, Screen};
use recite_writer_model::{View, Workbench, WorkbenchError};

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
    use_provide_context(move || mode);
    Router::<Location>::new(RouterConfig::default).into_element()
}
#[derive(Clone, PartialEq)]
struct Shell;
impl Component for Shell {
    fn render(&self) -> impl IntoElement {
        crate::workbench(use_consume::<AppMode>())
    }
}

fn snapshot(writer: Writer) -> Location {
    let model = writer.buffers.model.read();
    let state = writer.localisation.read();
    let files = writer.files.read();
    let root = files.as_ref().map(|f| f.root());
    let mut location = Location {
        screen: if *writer.pane.read() == Pane::Disk {
            Screen::Disk
        } else if *writer.pane.read() == Pane::Rename {
            Screen::Rename
        } else if *writer.pane.read() == Pane::Build {
            Screen::Build
        } else if *writer.pane.read() == Pane::Declarations {
            Screen::Declarations
        } else if *writer.pane.read() == Pane::Rules {
            Screen::Rules
        } else if *writer.pane.read() == Pane::Preview {
            Screen::Preview
        } else if !state.active {
            Screen::Write
        } else if state.view == crate::localisation::CatalogueView::Compare {
            Screen::Compare
        } else if state.view == crate::localisation::CatalogueView::Entry {
            Screen::Entry
        } else if state.view == crate::localisation::CatalogueView::Updates {
            Screen::Updates
        } else if state.view == crate::localisation::CatalogueView::Queue {
            Screen::Translations
        } else {
            Screen::Localise
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

fn select(model: &mut Workbench, location: &Location) -> Result<(), WorkbenchError> {
    if let (Some(passage), Some(beat)) = (&location.passage, &location.beat)
        && model
            .document()
            .find_passage(passage)?
            .is_none_or(|p| &p.section != beat)
    {
        return Err(recite_writer_model::EditError::Destination.into());
    }
    if location.screen == Screen::Rules {
        let passage = location
            .passage
            .as_deref()
            .ok_or(recite_writer_model::EditError::Destination)?;
        model.document().reply_rules(passage)?;
    }
    let view = if location.source || location.screen == Screen::Rules {
        Some(View::Source)
    } else if let Some(passage) = &location.passage {
        Some(View::Passage(passage.clone()))
    } else {
        location.beat.as_ref().map(|beat| View::Block(beat.clone()))
    };
    if let Some(view) = view {
        if model.view() != &view {
            model.select(view)?;
        }
    } else if model.view() == &View::Source {
        model.show_script()?;
    }
    Ok(())
}

fn apply(mut writer: Writer, location: &Location) -> Result<(), String> {
    let files = writer.files.peek();
    if let Some(project) = &location.project {
        let linked_root = recite_config::discover_project(project)
            .map_err(|error| format!("The linked project is unavailable: {error}"))?
            .manifest()
            .project_root()
            .to_owned();
        if files.as_ref().is_none_or(|f| f.root() != linked_root) {
            return Err(format!("Open the linked project first: {project}"));
        }
    }
    let path = location
        .catalogue
        .as_ref()
        .map(|catalogue| {
            files.as_ref().map_or_else(
                || Ok(std::path::PathBuf::from(catalogue)),
                |files| catalogue_path(files.root(), catalogue),
            )
        })
        .transpose()?;
    drop(files);
    writer.remember_scene();
    writer.try_navigate(|_| Ok(()))?;
    let current_catalogue = writer
        .localisation
        .peek()
        .catalogue
        .as_ref()
        .map(|c| c.path.clone());
    let catalogue_changed = current_catalogue != path;
    let catalogue = if catalogue_changed {
        if writer.localisation.peek().dirty() {
            return Err(crate::localisation::close_drafts_message());
        }
        if let Some(current) = writer.localisation.write().catalogue.as_mut() {
            current.flush_recovery()?;
        }
        path.as_ref()
            .map(|p| crate::localisation::catalogue::Catalogue::open_recoverable(p))
            .transpose()?
    } else {
        None
    };
    let current = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .map_err(|e| e.to_string())?
        .document()
        .key()
        .to_string();
    if !location.document.is_empty() && location.document != current {
        if writer.files.peek().is_some() {
            let path = writer
                .files
                .peek()
                .as_ref()
                .and_then(|project| project.path_for_document(&location.document))
                .ok_or("The linked scene is not in this project.")?;
            writer
                .buffers
                .switch(writer.files, &path, writer.dark, |m| select(m, location))?;
        } else {
            let index = recite_writer_model::WRITER_EXAMPLES
                .iter()
                .position(|e| e.name == location.document)
                .ok_or("The linked scene is not open.")?;
            crate::examples::select_at(writer, index, |model| select(model, location))?;
        }
    } else {
        writer.try_navigate(|m| select(m, location))?;
    }
    {
        let mut state = writer.localisation.write();
        if catalogue_changed {
            state.install(catalogue)?;
        }
        state.active = !matches!(
            location.screen,
            Screen::Write
                | Screen::Preview
                | Screen::Rules
                | Screen::Declarations
                | Screen::Build
                | Screen::Rename
                | Screen::Disk
        );
        state.entry_context = location.entry.clone();
        state.view = match location.screen {
            Screen::Translations => crate::localisation::CatalogueView::Queue,
            Screen::Updates => crate::localisation::CatalogueView::Updates,
            Screen::Compare => crate::localisation::CatalogueView::Compare,
            Screen::Entry => crate::localisation::CatalogueView::Entry,
            _ => crate::localisation::CatalogueView::Passage,
        };
        if location.screen == Screen::Updates {
            state.update_index = location.page;
        }
    }
    writer.queue.search.set(location.query.clone());
    writer.queue.attention.set(location.attention);
    if location.screen != Screen::Updates {
        writer.queue.page.set(location.page);
    }
    if location.screen == Screen::Rules {
        let passage = location
            .passage
            .as_deref()
            .ok_or("A reply-rules link needs a passage.")?;
        let retained = writer.buffers.rules.peek().as_ref().is_some_and(|r| {
            r.passage == passage
                && writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .is_ok_and(|m| r.belongs_to(m.document()))
                && r.source()
                    .is_ok_and(|source| source == writer.buffers.editor.peek().rope)
        });
        if !retained {
            crate::rules::open(writer, passage)?;
        }
    }
    if location.screen == Screen::Declarations {
        crate::declarations::try_open(writer)?;
    }
    if location.screen == Screen::Build {
        crate::builds::try_open(writer)?;
    }
    if location.screen == Screen::Rename {
        crate::rename::open(writer);
    }
    if location.screen == Screen::Disk {
        crate::external::try_open(writer)?;
    }
    writer.pane.set(if location.screen == Screen::Disk {
        Pane::Disk
    } else if location.screen == Screen::Rename {
        Pane::Rename
    } else if location.screen == Screen::Build {
        Pane::Build
    } else if location.screen == Screen::Declarations {
        Pane::Declarations
    } else if location.screen == Screen::Rules {
        Pane::Rules
    } else if location.screen == Screen::Preview {
        Pane::Preview
    } else if location.beat.is_some() || location.passage.is_some() {
        Pane::Script
    } else {
        Pane::Map
    });
    writer.layout.view.set(if location.source {
        recite_config::WriterView::Source
    } else {
        location.view.unwrap_or(recite_config::WriterView::Map)
    });
    writer.selection.set(location.beat.clone());
    writer.inspector_focus.request_focus();
    writer.restore_scroll();
    Ok(())
}

fn catalogue_path(root: &std::path::Path, relative: &str) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    let relative = std::path::Path::new(relative);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("The linked catalogue must be relative to the project.".into());
    }
    let path = std::fs::canonicalize(root.join(relative))
        .map_err(|error| format!("The linked catalogue is unavailable: {error}"))?;
    if !path.starts_with(root) {
        return Err("The linked catalogue is outside the project.".into());
    }
    Ok(path)
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
