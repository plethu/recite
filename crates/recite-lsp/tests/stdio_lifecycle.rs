mod support;

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

fn test_uri() -> String {
    file_uri(&Path::new(env!("CARGO_MANIFEST_DIR")).join("lifecycle-test.recite"))
}
