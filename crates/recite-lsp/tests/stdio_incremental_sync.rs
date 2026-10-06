mod support;
use serde_json::json;
use support::stdio::StdioHarness;

#[test]
fn malformed_wire_changes_preserve_text_and_version() {
    let range = json!({
        "start": {"line": 0, "character": 0},
        "end": {"line": 0, "character": 1}
    });
    let malformed = [
        json!({"range": true, "text": "replacement"}),
        json!({"range": {"start": {"line": 0, "character": 0}}, "text": "replacement"}),
        json!({"range": {"start": {"line": 0, "character": -1}, "end": {"line": 0, "character": 1}}, "text": "replacement"}),
        json!({"range": range, "rangeLength": -1, "text": "replacement"}),
        json!({"rangeLength": "invalid", "text": "replacement"}),
        json!({"rangeLength": 1, "text": "replacement"}),
    ];
    for change in malformed {
        let mut server = StdioHarness::start(json!({"capabilities": {}}));
        let uri = "file:///workspace/wire-changes.recite";
        server.did_open(uri, 1, ":: start\n-> target\n\n:: target\n-> END\n");
        assert!(
            server.diagnostics(uri)["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        server.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [
                    {"text": ":: poisoned\n-> END\n"},
                    change
                ]
            }),
        );
        let definition = server.request_result(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": uri}, "position": {"line": 1, "character": 4}
            }),
        );
        assert_eq!(
            definition["uri"], uri,
            "rejected batch changed accepted text: {change}"
        );
        assert_eq!(definition["range"]["start"]["line"], 3);

        // A valid edit at the rejected version must still be accepted. Unknown
        // extension fields remain legal; validation must preserve that freedom.
        server.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [{"text": ":: start\n-> missing\n", "extension": true}]
            }),
        );
        let diagnostics = server.diagnostics(uri);
        assert_eq!(diagnostics["version"], 2);
        assert!(!diagnostics["diagnostics"].as_array().unwrap().is_empty());
        server.finish();
    }
}

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
