//! Freya owns history; the workspace validates locations before accepting them.
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
        screen: if !state.active {
            Screen::Write
        } else if state.queue {
            Screen::Translations
        } else {
            Screen::Localise
        },
        catalogue: state.catalogue.as_ref().map(|c| {
            root.and_then(|root| c.path.strip_prefix(root).ok())
                .unwrap_or(&c.path)
                .to_string_lossy()
                .into_owned()
        }),
        project: root.map(|r| r.to_string_lossy().into_owned()),
        query: writer.queue.search.read().clone(),
        attention: *writer.queue.attention.read(),
        page: *writer.queue.page.read(),
        ..Location::default()
    };
    if let Ok(model) = model.as_ref() {
        location.document = model.document().key().to_string();
        location.source = model.view() == &View::Source;
        if !location.source && (*writer.pane.read() != Pane::Map || state.active) {
            location.beat = model.selected_block().ok().flatten();
            if let View::Passage(id) = model.view() {
                location.passage = Some(id.clone());
            }
        }
    }
    location
}

/// Observe accepted editor transitions, including scene navigation outside the toolbar.
/// Queue edits and focus within a beat replace the current location, so typing
/// does not flood Back history.
pub(super) fn track(writer: Writer, mode: AppMode) {
    let initial = use_try_consume::<InitialRoute>();
    let mut pending = use_state(move || initial.map(|r| r.0));
    let mut started = use_state(|| false);
    use_after_side_effect(move || {
        let ready = mode != AppMode::Project || writer.files.read().is_some();
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
                    let mut message = writer.message;
                    message.set(error);
                }
            }
            let _ = router.replace(snapshot(writer));
            return;
        }
        let next = snapshot(writer);
        let current = router.current::<Location>();
        if next != current {
            if next.same_place(&current) {
                let _ = router.replace(next);
            } else {
                let _ = router.push(next);
            }
        }
    });
}

pub(super) fn step(mut writer: Writer, forward: bool) {
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
        writer.message.set(error);
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
    let view = if location.source {
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
    if let Some(project) = &location.project {
        let files = writer.files.peek();
        if files
            .as_ref()
            .is_none_or(|f| f.root() != std::path::Path::new(project))
        {
            return Err(format!("Open the linked project first: {project}"));
        }
    }
    writer.navigate(|_| Ok(()));
    if !writer.message.peek().is_empty() {
        return Err(writer.message.peek().clone());
    }
    let path = location.catalogue.as_ref().map(|p| {
        writer
            .files
            .peek()
            .as_ref()
            .map_or_else(|| std::path::PathBuf::from(p), |f| f.root().join(p))
    });
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
        path.as_ref()
            .map(|p| crate::localisation::catalogue::Catalogue::open(p))
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
            if !writer.buffers.can_leave(writer.files.peek().as_ref()) {
                return Err(
                    "Save changes and apply or discard the draft before changing files.".into(),
                );
            }
            let mut files = writer.files.write();
            let project = files.as_mut().ok_or("Open the linked project first.")?;
            let path = project
                .path_for_document(&location.document)
                .ok_or("The linked scene is not in this project.")?;
            let next = project
                .select_at(&path, |m| select(m, location))
                .map_err(|e| e.to_string())?;
            writer.buffers.install(next, writer.dark);
        } else {
            let index = recite_writer_model::WRITER_EXAMPLES
                .iter()
                .position(|e| e.name == location.document)
                .ok_or("The linked scene is not open.")?;
            crate::examples::select_at(writer, index, |model| select(model, location))?;
        }
    } else {
        writer.navigate(|m| select(m, location));
        if !writer.message.peek().is_empty() {
            return Err(writer.message.peek().clone());
        }
    }
    {
        let mut state = writer.localisation.write();
        state.active = location.screen != Screen::Write;
        state.queue = location.screen == Screen::Translations;
        if catalogue_changed {
            state.catalogue = catalogue;
        }
    }
    writer.queue.search.set(location.query.clone());
    writer.queue.attention.set(location.attention);
    writer.queue.page.set(location.page);
    writer
        .pane
        .set(if location.beat.is_some() || location.passage.is_some() {
            Pane::Script
        } else {
            Pane::Map
        });
    writer.selection.set(location.beat.clone());
    writer.inspector_focus.request_focus();
    writer
        .scroll
        .scroll_to(ScrollPosition::Start, Direction::Vertical);
    Ok(())
}

pub(super) fn open_passage(
    mut writer: Writer,
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
            writer.message.set(error);
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
                writer.message.set(match Clipboard::set(link) {
                    Ok(()) => "Link copied.".into(),
                    Err(e) => format!("Could not copy link: {e:?}"),
                });
            },
        ))
        .into_element()
}

#[cfg(test)]
mod tests;
