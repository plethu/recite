mod support;
use serde_json::json;
use support::stdio::StdioHarness;

#[test]
fn dependent_changes_survive_coalescing_cancellation_and_reopen() {
    let mut server = StdioHarness::start(json!({"capabilities": {}}));
    assert_eq!(
        server.initialize()["capabilities"]["textDocumentSync"]["change"],
        2
    );
    let uri = "file:///workspace/incremental.recite";
    server.did_open(uri, 1, ":: start\n-> END\n# 😀0");
    assert!(
        server.diagnostics(uri)["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for version in 2..102 {
        let previous = (version - 2).to_string();
        server.notify("textDocument/didChange", json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"range": {"start": {"line": 2, "character": 4}, "end": {"line": 2, "character": 4 + previous.len()}}, "text": (version - 1).to_string()}]
        }));
        if version % 10 == 0 {
            let id = server.request(
                "textDocument/completion",
                json!({"textDocument": {"uri": uri}, "position": {"line": 1, "character": 3}}),
            );
            server.notify("$/cancelRequest", json!({"id": id}));
        }
    }
    server.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 102}, "contentChanges": [
        {"range": {"start": {"line": 2, "character": 4}, "end": {"line": 2, "character": 7}}, "rangeLength": 3, "text": "done"},
        {"range": {"start": {"line": 1, "character": 3}, "end": {"line": 1, "character": 6}}, "text": "missing"}
    ]}));
    loop {
        let diagnostics = server.diagnostics(uri);
        if diagnostics["version"] == 102 {
            assert!(!diagnostics["diagnostics"].as_array().unwrap().is_empty());
            break;
        }
    }
    server.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    );
    server.did_open(uri, 1, ":: start\n-> missing\n");
    server.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"range": {"start": {"line": 1, "character": 3}, "end": {"line": 1, "character": 10}}, "text": "END"}]}));
    loop {
        let diagnostics = server.diagnostics(uri);
        if diagnostics["version"] == 2 {
            assert!(diagnostics["diagnostics"].as_array().unwrap().is_empty());
            break;
        }
    }
    server.finish();
}
