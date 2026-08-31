use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::Builder;
use url::Url;

const READ_TIMEOUT: Duration = Duration::from_secs(2);

struct StdioHarness {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<io::Result<Value>>,
    next_id: u64,
}

impl StdioHarness {
    fn start(workspace_folders: &[&Path]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_recite-lsp"))
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| panic!("spawn recite-lsp binary: {error}"));
        let stdin = child
            .stdin
            .take()
            .unwrap_or_else(|| panic!("binary stdin is not piped"));
        let stdout = child
            .stdout
            .take()
            .unwrap_or_else(|| panic!("binary stdout is not piped"));
        let (sender, messages) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok(message) => {
                        if sender.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        let mut harness = Self {
            child,
            stdin,
            messages,
            next_id: 1,
        };
        let folders = workspace_folders
            .iter()
            .enumerate()
            .map(|(index, path)| {
                json!({
                    "uri": file_uri(path),
                    "name": format!("workspace-{index}")
                })
            })
            .collect::<Vec<_>>();
        let initialize_id = harness.request(
            "initialize",
            json!({
                "capabilities": {},
                "workspaceFolders": folders
            }),
        );
        harness.expect_response(initialize_id);
        harness.notify("initialized", json!({}));
        harness
    }

    fn request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message)
            .unwrap_or_else(|error| panic!("JSON-RPC message is serializable: {error}"));
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .unwrap_or_else(|error| panic!("write headers: {error}"));
        self.stdin
            .write_all(&body)
            .unwrap_or_else(|error| panic!("write JSON-RPC message: {error}"));
        self.stdin
            .flush()
            .unwrap_or_else(|error| panic!("flush JSON-RPC message: {error}"));
    }

    fn expect_response(&self, id: u64) {
        loop {
            let message = self.receive();
            if message.get("id") == Some(&json!(id)) {
                assert!(
                    message.get("error").is_none(),
                    "unexpected response: {message}"
                );
                return;
            }
        }
    }

    fn expect_diagnostics(&self, uri: &str) -> Value {
        loop {
            let message = self.receive();
            if message.get("method") != Some(&json!("textDocument/publishDiagnostics")) {
                continue;
            }
            if message["params"]["uri"] == uri {
                return message["params"].clone();
            }
        }
    }

    fn receive(&self) -> Value {
        self.messages
            .recv_timeout(READ_TIMEOUT)
            .unwrap_or_else(|error| panic!("stdio message within timeout: {error}"))
            .unwrap_or_else(|error| panic!("stdio message is valid JSON-RPC: {error}"))
    }

    fn finish(mut self) {
        let shutdown_id = self.request("shutdown", Value::Null);
        self.expect_response(shutdown_id);
        self.notify("exit", Value::Null);
        let mut exited = false;
        for _ in 0..400 {
            match self
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!("poll recite-lsp exit: {error}"))
            {
                Some(status) => {
                    assert!(
                        status.success(),
                        "recite-lsp exited unsuccessfully: {status}"
                    );
                    exited = true;
                    break;
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        assert!(exited, "recite-lsp did not exit within timeout");
    }
}

impl Drop for StdioHarness {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn stdio_workspace_folders_keep_manifest_and_fallback_saved_documents() {
    let temp = Builder::new()
        .prefix("recite % stdio workspace ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let manifest_root = temp.path().join("project");
    let fallback_root = temp.path().join("standalone");
    std::fs::create_dir_all(&manifest_root)
        .unwrap_or_else(|error| panic!("create project root: {error}"));
    std::fs::write(
        manifest_root.join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    )
    .unwrap_or_else(|error| panic!("write project manifest: {error}"));
    std::fs::create_dir_all(manifest_root.join("dialogue"))
        .unwrap_or_else(|error| panic!("create dialogue root: {error}"));
    std::fs::write(
        manifest_root.join("dialogue/project.recite"),
        ":: project\n",
    )
    .unwrap_or_else(|error| panic!("write manifest source: {error}"));
    std::fs::create_dir_all(&fallback_root)
        .unwrap_or_else(|error| panic!("create fallback root: {error}"));
    let fallback = fallback_root.join("standalone.recite");
    std::fs::write(&fallback, ":: standalone\n")
        .unwrap_or_else(|error| panic!("write fallback source: {error}"));

    let fallback_uri = file_uri(&fallback);
    let mut harness = StdioHarness::start(&[&manifest_root, &fallback_root]);
    std::fs::write(&fallback, "oops\n:: standalone\n")
        .unwrap_or_else(|error| panic!("write malformed fallback source: {error}"));
    harness.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": fallback_uri.clone() } }),
    );
    let published = harness.expect_diagnostics(&fallback_uri);
    assert!(
        !published["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty(),
        "the second workspace folder should retain a saved project document"
    );
    harness.finish();
}

#[test]
fn stdio_excluded_open_file_is_diagnosable_without_cross_project_diagnostics() {
    let temp = Builder::new()
        .prefix("recite % stdio excluded ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    std::fs::write(
        temp.path().join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    )
    .unwrap_or_else(|error| panic!("write project manifest: {error}"));
    std::fs::create_dir_all(temp.path().join("dialogue"))
        .unwrap_or_else(|error| panic!("create dialogue root: {error}"));
    std::fs::write(
        temp.path().join("dialogue/kept.recite"),
        ":: kept default\n> shared@83709c28414d0ce4659c\n  Kept.\n",
    )
    .unwrap_or_else(|error| panic!("write project source: {error}"));
    let excluded = temp.path().join("generated.recite");
    let excluded_uri = file_uri(&excluded);
    let mut harness = StdioHarness::start(&[temp.path()]);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": excluded_uri.clone(),
                "languageId": "recite",
                "version": 1,
                "text": ":: generated default\n> shared@83709c28414d0ce4659c\n  Generated.\n"
            }
        }),
    );
    let isolated = harness.expect_diagnostics(&excluded_uri);
    assert_eq!(
        isolated["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .len(),
        0,
        "an excluded buffer must not be merged with project documents: {isolated}"
    );

    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": excluded_uri.clone(), "version": 2 },
            "contentChanges": [{ "text": "oops\n" }]
        }),
    );
    let malformed = harness.expect_diagnostics(&excluded_uri);
    assert!(
        !malformed["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty(),
        "an excluded buffer still receives its own parser diagnostics"
    );
    harness.finish();
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "stdio closed"));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid content length: {error}"),
                )
            })?);
        }
    }
    let length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn file_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .unwrap_or_else(|()| {
            panic!(
                "path cannot be represented as a file URI: {}",
                path.display()
            )
        })
        .to_string()
}
