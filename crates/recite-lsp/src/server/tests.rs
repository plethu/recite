use super::{
    Server,
    requests::{RequestState, StopReason},
    workers::QueryResult,
};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::InitializeParams;
use recite_compiler::authoring::CancellationToken;
use serde_json::json;
use std::time::Duration;

fn server() -> (Server, Connection) {
    let (connection, client) = Connection::memory();
    let catalog = recite_ui::UiCatalog::load(&recite_ui::UiLocale::default())
        .unwrap_or_else(|error| panic!("catalog: {error}"));
    (
        Server::new(connection, InitializeParams::default(), catalog),
        client,
    )
}
fn completion(id: i32, uri: &str) -> Request {
    Request::new(
        id.into(),
        "textDocument/completion".to_owned(),
        json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 0}}),
    )
}
fn code(server: &Server, id: i32) -> i32 {
    server
        .requests
        .get(&RequestId::from(id))
        .and_then(|pending| pending.response(&id.into()))
        .and_then(|response| response.error)
        .map(|error| error.code)
        .unwrap_or_else(|| panic!("expected error response"))
}
#[test]
fn cancellation_wins_before_dispatch_during_work_and_after_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    for id in 1..=3 {
        server.request(completion(id, "untitled:test"))?;
    }
    let running = server
        .requests
        .get_mut(&2.into())
        .unwrap_or_else(|| panic!("request"));
    running.dispatch();
    let token = running.control.clone();
    let serial = running.serial;
    server
        .requests
        .get_mut(&3.into())
        .unwrap_or_else(|| panic!("request"))
        .dispatch();
    server.query_finished(QueryResult {
        serial: server.requests[&3.into()].serial,
        result: Ok(json!(["candidate"])),
    });
    for id in 1..=3 {
        server.cancel(json!({"id": id}));
        assert_eq!(code(&server, id), -32800);
    }
    assert!(token.checkpoint().is_err());
    server.query_finished(QueryResult {
        serial,
        result: Ok(json!(["late candidate"])),
    });
    assert_eq!(code(&server, 2), -32800);
    server.cancel(json!({"id": 99}));
    assert_eq!(server.requests.len(), 3);
    server.workers.join()?;
    Ok(())
}
#[test]
fn late_completion_cannot_attach_to_a_reused_protocol_id() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut server, _client) = server();
    server.request(completion(1, "untitled:test"))?;
    let old_serial = server.requests[&1.into()].serial;
    server.cancel(json!({"id": 1}));
    server.requests.remove(&1.into()); // Exactly the transport handoff's ownership release.
    server.request(completion(1, "untitled:test"))?;
    server.query_finished(QueryResult {
        serial: old_serial,
        result: Ok(json!(["old"])),
    });
    assert!(matches!(
        server.requests[&1.into()].state,
        RequestState::Queued(_)
    ));
    let new_serial = server.requests[&1.into()].serial;
    server
        .requests
        .get_mut(&1.into())
        .unwrap_or_else(|| panic!("request"))
        .dispatch();
    server.query_finished(QueryResult {
        serial: new_serial,
        result: Ok(json!(["new"])),
    });
    assert_eq!(
        server.requests[&1.into()]
            .response(&1.into())
            .and_then(|r| r.result),
        Some(json!(["new"]))
    );
    server.workers.join()?;
    Ok(())
}
#[test]
fn partition_changes_preserve_unrelated_queries_and_cancellation_overrides_staleness()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    server
        .scopes
        .insert("file:///a.recite".to_owned(), "a".to_owned());
    server
        .scopes
        .insert("file:///b.recite".to_owned(), "b".to_owned());
    server.request(completion(1, "file:///a.recite"))?;
    server.request(completion(2, "file:///b.recite"))?;
    server.request(completion(3, "untitled:unknown"))?;
    assert!(server.epochs.advance(Some("a")).is_some());
    server.invalidate();
    assert_eq!(code(&server, 1), -32803);
    assert!(matches!(
        server.requests[&2.into()].state,
        RequestState::Queued(_)
    ));
    assert_eq!(code(&server, 3), -32803);
    server.cancel(json!({"id": 1}));
    assert_eq!(code(&server, 1), -32800);
    assert!(server.epochs.advance(None).is_some());
    server.invalidate();
    assert_eq!(code(&server, 2), -32803);
    server.workers.join()?;
    Ok(())
}
#[test]
fn full_sync_coalescing_preserves_lifecycle_boundaries_and_rejects_old_versions()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    let uri = "untitled:buffer";
    server.notification(Notification::new("textDocument/didOpen".to_owned(), json!({"textDocument": {"uri": uri, "languageId": "recite", "version": 1, "text": ":: first"}})))?;
    for version in [2, 3, 2] {
        server.notification(Notification::new("textDocument/didChange".to_owned(), json!({"textDocument": {"uri": uri, "version": version}, "contentChanges": [{"text": format!(":: v{version}")}]})))?;
    }
    assert_eq!(server.updates.len(), 2);
    assert_eq!(server.revision, 3);
    server.notification(Notification::new(
        "textDocument/didClose".to_owned(),
        json!({"textDocument": {"uri": uri}}),
    ))?;
    server.notification(Notification::new("textDocument/didOpen".to_owned(), json!({"textDocument": {"uri": uri, "languageId": "recite", "version": 1, "text": ":: reopened"}})))?;
    assert_eq!(server.updates.len(), 4);
    server.workers.join()?;
    Ok(())
}
#[test]
fn query_capacity_is_bounded_and_shutdown_stops_all_accepted_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    for id in 1..=65 {
        server.request(completion(id, "untitled:test"))?;
    }
    assert_eq!(server.requests.len(), 64);
    let Message::Response(response) = &server.output[0].message else {
        panic!("busy response");
    };
    assert_eq!(
        response.error.as_ref().and_then(|e| e.data.as_ref()),
        Some(&json!({"reason": "server_busy"}))
    );
    server.request(Request::new(100.into(), "shutdown".to_owned(), ()))?;
    assert!(server.requests.values().all(|pending| matches!(
        pending.state,
        RequestState::Stopped(StopReason::Shutdown)
    ) && pending.control.checkpoint().is_err()));
    server.workers.join()?;
    Ok(())
}
#[test]
fn blocked_writer_does_not_prevent_cancellation_at_final_handoff()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, client) = server();
    let (sender, responses) = crossbeam_channel::bounded(0);
    server.connection.sender = sender;
    // Hold initial analysis at a coordinator boundary; no CPU timing is involved.
    server.analyzing = Some(CancellationToken::new());
    server.request(completion(1, "untitled:test"))?;
    server
        .requests
        .get_mut(&1.into())
        .unwrap_or_else(|| panic!("request"))
        .state = RequestState::Ready(Response::new_ok(1.into(), json!(["candidate"])));
    client
        .sender
        .send(Notification::new("$/cancelRequest".to_owned(), json!({"id": 1})).into())?;
    client
        .sender
        .send(Request::new(2.into(), "shutdown".to_owned(), ()).into())?;
    client
        .sender
        .send(Notification::new("exit".to_owned(), ()).into())?;
    let thread = std::thread::spawn(move || server.run());
    let Message::Response(response) = responses.recv_timeout(Duration::from_secs(2))? else {
        panic!("cancel response");
    };
    assert_eq!(response.error.map(|error| error.code), Some(-32800));
    let Message::Response(response) = responses.recv_timeout(Duration::from_secs(2))? else {
        panic!("shutdown response");
    };
    assert_eq!(response.id, 2.into());
    thread.join().unwrap_or_else(|_| panic!("server panic"))?;
    assert!(responses.try_recv().is_err());
    Ok(())
}

#[test]
fn superseded_close_diagnostics_are_recomputed_before_publication()
-> Result<(), Box<dyn std::error::Error>> {
    use super::{
        freshness::Epochs,
        workers::{AnalysisResult, AnalysisSnapshot},
    };
    use std::{collections::BTreeMap, sync::Arc};
    for reopen in [false, true] {
        let (mut server, _client) = server();
        let catalog = recite_ui::UiCatalog::load(&recite_ui::UiLocale::default())?;
        let workspace = crate::workspace::LspWorkspace::with_ui_catalog(
            crate::workspace::WorkspaceConfig::from_initialize_params(&InitializeParams::default()),
            catalog,
        )?;
        let uri: lsp_types::Uri = "untitled:closed".parse()?;
        if reopen {
            server.notification(Notification::new("textDocument/didOpen".to_owned(), json!({"textDocument": {"uri": uri, "languageId": "recite", "version": 5, "text": ":: reopened\n-> END\n"}})))?;
        } else {
            assert!(server.epochs.advance(None).is_some());
        }
        server.analysis_finished(AnalysisResult {
            through: 0,
            epochs: Epochs::default(),
            result: Ok(Some(AnalysisSnapshot {
                workspace: Arc::new(workspace),
                scopes: BTreeMap::new(),
                diagnostics: vec![lsp_types::PublishDiagnosticsParams {
                    uri: uri.clone(),
                    version: None,
                    diagnostics: Vec::new(),
                }],
            })),
        })?;
        assert!(
            server.output.is_empty(),
            "old close cannot pass the new fence"
        );
        assert!(server.dirty_diagnostics.contains(&uri));
        server.schedule()?;
        let completed = server
            .workers
            .analyzed
            .recv_timeout(Duration::from_secs(2))?;
        server.analysis_finished(completed)?;
        assert!(server.dirty_diagnostics.is_empty());
        let publications = server
            .output
            .iter()
            .filter_map(|publication| match &publication.message {
                Message::Notification(notification)
                    if notification.method == "textDocument/publishDiagnostics" =>
                {
                    Some(&notification.params)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(publications.len(), 1);
        assert_eq!(publications[0]["uri"], uri.as_str());
        assert_eq!(
            publications[0]
                .get("version")
                .and_then(serde_json::Value::as_i64),
            if reopen { Some(5) } else { None }
        );
        server.workers.join()?;
    }
    Ok(())
}

#[test]
fn hot_partition_cannot_repeatedly_restart_bootstrap_or_sibling_analysis()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    let bootstrap = CancellationToken::new();
    server.analyzing = Some(bootstrap.clone());
    for name in ["a", "b"] {
        server.notification(Notification::new("textDocument/didOpen".to_owned(), json!({"textDocument": {"uri": format!("untitled:{name}"), "languageId": "recite", "version": 1, "text": ":: first"}})))?;
        server
            .scopes
            .insert(format!("untitled:{name}"), name.to_owned());
    }
    assert!(bootstrap.checkpoint().is_ok());
    server.updates.clear();
    server.analyzed_epochs = server.epochs.clone();
    server.analysis_partition = Some("a".to_owned());
    for (name, version) in [("a", 2), ("b", 2), ("a", 3)] {
        let running = CancellationToken::new();
        server.analyzing = Some(running.clone());
        server.notification(Notification::new("textDocument/didChange".to_owned(), json!({"textDocument": {"uri": format!("untitled:{name}"), "version": version}, "contentChanges": [{"text": ":: changed"}]})))?;
        assert_eq!(running.checkpoint().is_err(), name == "a" && version == 2);
    }
    server.workers.join()?;
    Ok(())
}

#[test]
fn exhaustion_and_worker_loss_fail_explicitly_without_reusing_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut exhausted, _client) = server();
    exhausted.serial = u64::MAX;
    assert!(matches!(
        exhausted.request(completion(1, "untitled:test")),
        Err(super::ServerError::SequenceExhausted)
    ));
    assert!(exhausted.requests.is_empty());
    exhausted.workers.join()?;
    let (mut lost, _client) = server();
    let (sender, receiver) = crossbeam_channel::bounded(1);
    drop(sender);
    lost.workers.analyzed = receiver;
    assert!(matches!(lost.run(), Err(super::ServerError::WorkerPanic)));
    Ok(())
}

#[test]
fn unresolved_topology_does_not_reuse_cached_partition_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    server
        .scopes
        .insert("file:///moved.recite".to_owned(), "old-project".to_owned());
    server.scopes.insert(
        "file:///sibling.recite".to_owned(),
        "new-project".to_owned(),
    );
    assert!(server.epochs.advance(None).is_some()); // A manifest/symlink watcher is pending.
    assert_eq!(server.known_scope(&"file:///moved.recite".parse()?), None);
    server.request(completion(1, "file:///sibling.recite"))?;
    assert!(matches!(
        server.request(Request::new(1.into(), "shutdown".to_owned(), ())),
        Err(super::ServerError::DuplicateRequest)
    ));
    assert!(!server.shutdown_requested);
    assert!(server.epochs.advance(Some("old-project")).is_some());
    server.invalidate();
    assert_eq!(
        code(&server, 1),
        -32803,
        "unresolved ownership must depend on all input changes"
    );
    server.workers.join()?;
    Ok(())
}

#[test]
fn deferred_diagnostics_preserve_publication_order() -> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    let canonical: lsp_types::Uri = "file:///project/schema.json".parse()?;
    let alias: lsp_types::Uri = "file:///project/./schema.json".parse()?;
    for uri in [&canonical, &alias, &canonical] {
        server.enqueue(super::Publication {
            message: Notification::new(
                "textDocument/publishDiagnostics".to_owned(),
                json!({"uri": uri, "diagnostics": []}),
            )
            .into(),
            fence: Some(server.epochs.fence(None)),
        })?;
    }
    assert!(server.epochs.advance(None).is_some());
    server.prune_publications();
    assert_eq!(server.dirty_diagnostics, vec![canonical, alias]);
    assert!(server.output.is_empty());
    server.workers.join()?;
    Ok(())
}

#[test]
fn diagnostic_batch_backpressure_preserves_cancellation_and_reanalysis()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, _client) = server();
    for index in 0..300 {
        server.enqueue(super::Publication {
            message: Notification::new(
                "textDocument/publishDiagnostics".to_owned(),
                json!({"uri": format!("untitled:batch-{index}"), "diagnostics": []}),
            )
            .into(),
            fence: Some(server.epochs.fence(None)),
        })?;
    }
    server.schedule()?;
    assert!(
        server.analyzing.is_none(),
        "a second batch must wait for the first"
    );
    server.request(completion(1, "untitled:batch-0"))?;
    server.cancel(json!({"id": 1}));
    assert_eq!(code(&server, 1), -32800);
    server.epochs.advance(None).ok_or("epoch overflow")?;
    server.prune_publications();
    assert!(server.output.is_empty());
    assert_eq!(server.dirty_diagnostics.len(), 300);
    server.schedule()?;
    assert!(
        server.analyzing.is_some(),
        "stale batch must not block fresh analysis"
    );
    if let Some(control) = &server.analyzing {
        control.interrupt();
    }
    server.workers.join()?;
    Ok(())
}

#[test]
fn pipelined_errors_apply_ingress_backpressure_without_disconnect()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut server, client) = server();
    let (sender, responses) = crossbeam_channel::bounded(0);
    server.connection.sender = sender;
    // Queue input before starting the coordinator, making the 32-to-1 ingress
    // imbalance deterministic. The test thread continuously consumes output.
    for id in 1..=600 {
        client
            .sender
            .send(Request::new(id.into(), "unsupported".to_owned(), ()).into())?;
    }
    let thread = std::thread::spawn(move || server.run());
    for id in 1..=600 {
        let Message::Response(response) = responses.recv_timeout(Duration::from_secs(2))? else {
            panic!("expected error response");
        };
        assert_eq!(response.id, id.into());
        assert_eq!(response.error.map(|error| error.code), Some(-32601));
    }
    client
        .sender
        .send(Request::new(601.into(), "shutdown".to_owned(), ()).into())?;
    let Message::Response(response) = responses.recv_timeout(Duration::from_secs(2))? else {
        panic!("expected shutdown response");
    };
    assert_eq!(response.id, 601.into());
    assert!(response.error.is_none());
    client
        .sender
        .send(Notification::new("exit".to_owned(), ()).into())?;
    thread.join().map_err(|_| "server panicked")??;
    Ok(())
}
