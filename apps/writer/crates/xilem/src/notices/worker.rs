use super::{Connection, Event, Output, Request, WatchScope, Writer, listener};
use listener::Listener;
use recite_writer_session::external::{Affected, Changes, Subscription, Watch};
use tokio::{
    sync::{mpsc, watch},
    task::spawn_blocking,
};
use xilem::{
    ViewCtx, WindowId,
    core::{MessageProxy, NoElement, View},
};
#[cfg(test)]
mod tests;

pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::task(
        |proxy, writer: &mut Writer| {
            let source = writer.session.project().map(|project| {
                project
                    .files()
                    .watch
                    .as_ref()
                    .map(Watch::subscribe)
                    .map_err(Clone::clone)
            });
            let (send, requests) = mpsc::channel(1);
            let (catalogue, binding) = watch::channel(None);
            writer.notices.connection = Some(Connection {
                requests: send,
                catalogue,
            });
            if source.is_some() {
                writer.notices.pending.sources = Affected::AllOpen;
            }
            writer.resume_external_notices();
            serve(proxy, writer.window, source, requests, binding)
        },
        Writer::accept_external_notice,
    )
}
struct Catalogue {
    scope: WatchScope,
    _watch: Watch,
    listener: Listener,
}
async fn bind(scope: WatchScope) -> Result<Catalogue, String> {
    spawn_blocking(move || {
        let watch = Watch::parent(scope.path())?;
        Ok((scope, watch))
    })
    .await
    .map_err(|error| error.to_string())?
    .map(|(scope, watch)| Catalogue {
        scope,
        listener: Listener::new(watch.subscribe()),
        _watch: watch,
    })
}
async fn catalogue_changes(catalogue: &mut Option<Catalogue>) -> Option<(WatchScope, Changes)> {
    match catalogue {
        Some(catalogue) => catalogue
            .listener
            .changes
            .recv()
            .await
            .map(|changes| (catalogue.scope.clone(), changes)),
        None => std::future::pending().await,
    }
}
async fn serve(
    proxy: MessageProxy<Output>,
    window: WindowId,
    source: Option<Result<Subscription, String>>,
    mut requests: mpsc::Receiver<Request>,
    mut binding: watch::Receiver<Option<WatchScope>>,
) {
    let emit = |event| proxy.message(Output { window, event }).is_ok();
    let mut source = match source {
        Some(Ok(source)) => Some(Listener::new(source)),
        Some(Err(error)) => {
            if !emit(Event::Warning(error)) {
                return;
            }
            None
        }
        None => None,
    };
    let mut catalogue = None;
    loop {
        let event = tokio::select! {
            changes = listener::next(&mut source) => match changes {
                Some(changes) => Event::Sources(changes),
                None => { source = None; continue; }
            },
            changes = catalogue_changes(&mut catalogue) => match changes {
                Some((scope, changes)) => Event::Catalogue(scope, changes),
                None => { catalogue = None; continue; }
            },
            changed = binding.changed() => {
                if changed.is_err() { break; }
                let scope = binding.borrow_and_update().clone();
                catalogue = None;
                let Some(scope) = scope else { continue; };
                let warning = match bind(scope.clone()).await {
                    Ok(bound) => { catalogue = Some(bound); None }
                    Err(error) => Some(error),
                };
                Event::Bound(scope, warning)
            }
            request = requests.recv() => {
                let Some(request) = request else { break; };
                match spawn_blocking(move || request.observe()).await {
                    Ok(observed) => Event::Observed(observed),
                    Err(error) => {
                        emit(Event::Stopped(format!("Automatic disk-change checks stopped: {error}. Save still checks for conflicts.")));
                        break;
                    }
                }
            }
        };
        if !emit(event) {
            break;
        }
    }
}
