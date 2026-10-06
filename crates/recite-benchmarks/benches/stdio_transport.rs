//! Exercise the upstream stdio transport with real framing and JSON destruction.
use lsp_server::{Connection, Message, Notification, Response};
use serde::Serialize;
use serde_json::{Value, json};
use std::{error::Error, io, time::Instant};

#[derive(Serialize)]
struct Sample {
    serial: i64,
    notification_handoff_ms: f64,
    response_handoff_ms: f64,
}

#[allow(
    clippy::disallowed_methods,
    reason = "benchmark instrumentation measures native transport scheduling outside deterministic language code"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::var("RECITE_STDIO_PROBE_MODE")?;
    if !matches!(mode.as_str(), "native" | "direct") {
        return Err("expected native or direct".into());
    }
    // The direct variant is a diagnostic framing baseline, not a proposed
    // production transport: it deliberately has no reader/writer/dropper threads.
    let transport = (mode == "native").then(Connection::stdio);
    let mut stdin = (mode == "direct").then(|| io::stdin().lock());
    let mut stdout = (mode == "direct").then(|| io::stdout().lock());
    let mut samples = Vec::new();
    loop {
        let message = match &transport {
            Some((connection, _)) => connection.receiver.recv().ok(),
            None => Message::read(stdin.as_mut().ok_or("missing direct input")?)?,
        };
        let Some(message) = message else { break };
        let Message::Request(request) = message else {
            if matches!(message, Message::Notification(ref n) if n.method == "exit") {
                break;
            }
            return Err("probe expected request or exit".into());
        };
        let mut send = |message: Message| -> Result<(), Box<dyn Error>> {
            match &transport {
                Some((connection, _)) => connection.sender.send(message)?,
                None => message.write(stdout.as_mut().ok_or("missing direct output")?)?,
            }
            Ok(())
        };
        match request.method.as_str() {
            "shutdown" => send(Response::new_ok(request.id, Value::Null).into())?,
            "probe/report" => send(Response::new_ok(request.id, &samples).into())?,
            "probe/exchange" => {
                let serial = request.params["serial"].as_i64().ok_or("missing serial")?;
                let entries = request.params["entries"]
                    .as_u64()
                    .ok_or("missing entries")?;
                if entries > 4096 || samples.len() >= 2000 {
                    return Err("probe limits exceeded".into());
                }
                let payload: Vec<_> = (0..entries)
                    .map(|index| json!({"range": {"start": {"line": index, "character": 0},
                        "end": {"line": index, "character": 10}}, "message": "probe diagnostic", "severity": 1}))
                    .collect();
                let started = Instant::now();
                send(Notification::new("probe/diagnostics".into(), payload).into())?;
                let notification_handoff_ms = started.elapsed().as_secs_f64() * 1000.0;
                let started = Instant::now();
                send(Response::new_ok(request.id, json!({"serial": serial})).into())?;
                samples.push(Sample {
                    serial,
                    notification_handoff_ms,
                    response_handoff_ms: started.elapsed().as_secs_f64() * 1000.0,
                });
            }
            _ => return Err("unknown probe method".into()),
        }
    }
    drop(stdin);
    drop(stdout);
    if let Some((connection, threads)) = transport {
        drop(connection);
        threads.join()?;
    }
    Ok(())
}
