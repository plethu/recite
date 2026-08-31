use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use url::Url;

const READ_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct StdioHarness {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<io::Result<Value>>,
    next_id: u64,
}

impl StdioHarness {
    pub(crate) fn start(params: Value) -> Self {
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
        let initialize_id = harness.request("initialize", params);
        harness.expect_response(initialize_id);
        harness.notify("initialized", json!({}));
        harness
    }

    pub(crate) fn request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    pub(crate) fn notify(&mut self, method: &str, params: Value) {
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

    pub(crate) fn expect_diagnostics(&self, uri: &str) -> Value {
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

    pub(crate) fn barrier(&mut self, uri: &str) -> Vec<Value> {
        let request_id = self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 0 }
            }),
        );
        let mut messages = Vec::new();
        loop {
            let message = self.receive();
            if message.get("id") == Some(&json!(request_id)) {
                assert!(
                    message.get("error").is_none(),
                    "barrier request failed: {message}"
                );
                return messages;
            }
            messages.push(message);
        }
    }

    fn receive(&self) -> Value {
        self.messages
            .recv_timeout(READ_TIMEOUT)
            .unwrap_or_else(|error| panic!("stdio message within timeout: {error}"))
            .unwrap_or_else(|error| panic!("stdio message is valid JSON-RPC: {error}"))
    }

    pub(crate) fn finish(mut self) {
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

pub(crate) fn file_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .unwrap_or_else(|()| {
            panic!(
                "path cannot be represented as a file URI: {}",
                path.display()
            )
        })
        .to_string()
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
