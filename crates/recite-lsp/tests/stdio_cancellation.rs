mod support;
use serde_json::json;
use support::stdio::{StdioHarness, file_uri};

#[test]
fn cancellation_settles_once_and_preserves_following_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let source = (0..1500)
        .map(|index| format!(":: block_{index}\n-> block_{index}\n"))
        .collect::<String>();
    let path = project.path().join("dialogue.recite");
    std::fs::write(&path, &source)?;
    let uri = file_uri(&path);
    let mut harness =
        StdioHarness::start(json!({"capabilities": {}, "rootUri": file_uri(project.path())}));
    harness.did_open(&uri, 1, &source);
    let mut requests = Vec::new();
    for _ in 0..16 {
        let id = harness.request("textDocument/rename", json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 4}, "newName": "renamed"}));
        harness.notify("$/cancelRequest", json!({"id": id}));
        requests.push(id);
    }
    let mut cancelled = 0;
    for id in &requests {
        let response = harness.response_message(*id);
        if response.get("error").is_some() {
            assert_eq!(response["error"]["code"], -32800);
            assert!(response.get("result").is_none());
            cancelled += 1;
        } else {
            // Completion may legitimately beat an incoming cancellation. Exact
            // cutpoint races are separately driven by coordinator unit tests.
            assert!(response["result"]["documentChanges"].is_array());
        }
    }
    assert!(
        cancelled > 0,
        "the real stdio cancellation path must be exercised"
    );
    let following = harness.request_result("textDocument/rename", json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 4}, "newName": "after_cancel"}));
    assert!(following["documentChanges"].is_array());
    for id in &requests {
        harness.notify("$/cancelRequest", json!({"id": id}));
    }
    harness.notify("$/cancelRequest", json!({"id": "unknown"}));
    let messages = harness.barrier(&uri);
    assert!(
        messages.iter().all(|message| message.get("id").is_none()),
        "late or duplicate response: {messages:?}"
    );
    harness.finish();
    Ok(())
}
