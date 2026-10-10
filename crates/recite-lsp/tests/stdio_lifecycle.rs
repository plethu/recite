mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use serde_json::json;
use support::stdio::{StdioHarness, file_uri};

#[test]
fn stdio_dynamic_registration_waits_for_initialized_and_is_not_duplicated() {
    let mut harness = StdioHarness::start_uninitialized(json!({
        "capabilities": {
            "workspace": {
                "didChangeWatchedFiles": { "dynamicRegistration": true }
            }
        }
    }));
    harness.assert_no_messages();

    harness.send_initialized();
    let registration = harness.receive_message();
    assert_eq!(registration["method"], "client/registerCapability");
    let params = registration
        .get("params")
        .unwrap_or_else(|| panic!("registration params are missing: {registration}"));
    let registrations = params["registrations"]
        .as_array()
        .unwrap_or_else(|| panic!("registrations array is missing: {registration}"));
    assert_eq!(registrations.len(), 1);
    assert_eq!(registrations[0]["id"], "recite-project-discovery");
    assert_eq!(
        registrations[0]["method"],
        "workspace/didChangeWatchedFiles"
    );
    assert_eq!(
        registrations[0]["registerOptions"]["watchers"][0]["globPattern"],
        "**/*"
    );
    assert_eq!(
        registrations[0]["registerOptions"]["watchers"][0]["kind"],
        7
    );

    harness.send_initialized();
    let duplicate_messages = harness.barrier(&test_uri());
    assert!(
        duplicate_messages
            .iter()
            .all(|message| message["method"] != "client/registerCapability"),
        "duplicate initialized sent another registration: {duplicate_messages:?}"
    );
    harness.finish();
}

#[test]
fn stdio_static_only_client_does_not_receive_dynamic_registration() {
    let mut harness = StdioHarness::start(json!({ "capabilities": {} }));

    let messages = harness.barrier(&test_uri());
    assert!(
        messages
            .iter()
            .all(|message| message["method"] != "client/registerCapability"),
        "static-only client received dynamic registration: {messages:?}"
    );
    harness.finish();
}

#[test]
fn silence_assertions_reject_notifications_already_queued_with_a_response() {
    let mut harness = StdioHarness::start(json!({ "capabilities": {} }));
    let uri = test_uri();
    harness.did_open(
        &uri,
        1,
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
    );
    let _completion = harness.request_result(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 0 }
        }),
    );

    let result = catch_unwind(AssertUnwindSafe(|| harness.assert_no_message()));
    assert!(
        result.is_err(),
        "a pending notification must not be hidden by a channel poll"
    );
    harness.finish();
}

#[test]
fn closing_one_untitled_buffer_clears_its_uri_and_retains_its_sibling() {
    let closed = "untitled:z";
    let sibling = "untitled:a";
    let mut harness = StdioHarness::start(json!({ "capabilities": {} }));
    harness.did_open(closed, 1, "oops\n:: closed\n-> END\n");
    assert!(harness.barrier(closed).iter().any(|message| {
        message["params"]["uri"] == closed
            && message["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty())
    }));
    harness.did_open(sibling, 1, ":: sibling\n-> END\n");
    harness.barrier(sibling);

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": closed } }),
    );

    let messages = harness.barrier(sibling);
    assert_eq!(messages.len(), 2, "close publications: {messages:?}");
    assert_eq!(messages[0]["params"]["uri"], closed);
    assert!(messages[0]["params"]["version"].is_null());
    assert_eq!(messages[0]["params"]["diagnostics"], json!([]));
    assert_eq!(messages[1]["params"]["uri"], sibling);
    assert_eq!(messages[1]["params"]["version"], 1);
    assert_eq!(messages[1]["params"]["diagnostics"], json!([]));
    harness.did_change(sibling, 2, ":: changed\n-> END\n");
    let changed = harness.barrier(sibling);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0]["params"]["uri"], sibling);
    assert_eq!(changed[0]["params"]["version"], 2);
    harness.finish();
}

fn test_uri() -> String {
    file_uri(&Path::new(env!("CARGO_MANIFEST_DIR")).join("lifecycle-test.recite"))
}
