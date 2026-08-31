use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;

const READ_TIMEOUT: Duration = Duration::from_secs(2);

struct StdioHarness {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<io::Result<Value>>,
    next_id: u64,
}

impl StdioHarness {
    fn start(root: &Path, schema: &Path) -> Self {
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
        let initialize_id = harness.request(
            "initialize",
            json!({
                "capabilities": {},
                "rootUri": file_uri(root),
                "initializationOptions": { "schema": schema.display().to_string() }
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
fn stdio_schema_alias_close_clears_alias_and_refreshes_canonical() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let schema = temp.path().join("standalone.toml");
    let alias = temp.path().join(".").join("standalone.toml");
    std::fs::write(
        &schema,
        "schema_version = 1\n[producer]\nid = \"dialogue\"\n",
    )
    .unwrap_or_else(|error| panic!("write standalone schema: {error}"));
    let canonical_uri = file_uri(&schema);
    let alias_uri = file_uri(&alias);
    let mut harness = StdioHarness::start(temp.path(), &schema);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": alias_uri.clone(),
                "languageId": "toml",
                "version": 7,
                "text": "not a schema\n"
            }
        }),
    );
    let invalid = harness.expect_diagnostics(&alias_uri);
    assert_eq!(invalid["version"], 7);
    assert!(
        !invalid["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
    );

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": alias_uri.clone() } }),
    );
    let alias_clear = harness.expect_diagnostics(&alias_uri);
    assert!(alias_clear["version"].is_null());
    assert!(
        alias_clear["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
    );
    let canonical_refresh = harness.expect_diagnostics(&canonical_uri);
    assert!(canonical_refresh["version"].is_null());
    assert!(
        canonical_refresh["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
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
    format!("file://{}", path.display())
}
