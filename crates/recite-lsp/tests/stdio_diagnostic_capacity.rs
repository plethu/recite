mod support;

use serde_json::json;
use support::stdio::{StdioHarness, file_uri};

#[test]
fn consuming_client_drains_diagnostic_batches_larger_than_control_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let mut harness =
        StdioHarness::start(json!({"capabilities": {}, "rootUri": file_uri(project.path())}));
    for index in 0..270 {
        let uri = format!("untitled:capacity-{index:03}");
        harness.did_open(&uri, 1, ":: start default\n-> END\n");
        let messages = harness.barrier(&uri);
        assert!(messages.iter().any(|message| message["params"]["uri"] == uri
            && message["params"]["version"] == 1));
    }
    harness.did_change("untitled:capacity-000", 2, ":: changed default\n-> END\n");
    let messages = harness.barrier("untitled:capacity-000");
    assert!(messages.iter().any(
        |message| message["params"]["uri"] == "untitled:capacity-000"
            && message["params"]["version"] == 2
    ));
    harness.finish();
    Ok(())
}

#[test]
fn consuming_client_pipelines_errors_and_continues_using_the_server() {
    let mut harness = StdioHarness::start(json!({"capabilities": {}}));
    // StdioHarness reads stdout on its own thread throughout this input burst.
    let requests = (0..600)
        .map(|_| harness.request("recite/unsupported", json!({})))
        .collect::<Vec<_>>();
    for id in requests {
        assert_eq!(harness.response_message(id)["error"]["code"], -32601);
    }
    harness.did_open("untitled:after-pipeline", 1, ":: start default\n-> END\n");
    harness.barrier("untitled:after-pipeline");
    harness.finish();
}

#[test]
fn consuming_client_pipelines_more_opens_than_the_update_capacity() {
    let mut harness = StdioHarness::start(json!({"capabilities": {}}));
    // One analysis batch includes hundreds of lifecycle transitions in a debug
    // build. This is a delivery assertion, not a latency threshold.
    harness.set_read_timeout(std::time::Duration::from_secs(60));
    for index in 0..270 {
        harness.did_open(
            &format!("untitled:pipeline-{index:03}"),
            1,
            ":: start default\n-> END\n",
        );
    }
    let messages = harness.barrier("untitled:pipeline-269");
    assert!(messages.iter().any(
        |message| message["params"]["uri"] == "untitled:pipeline-269"
            && message["params"]["version"] == 1
    ));
    harness.finish();
}
