//! Isolate the channel operations used by the LSP workers and stdio transport.
//! Run through scripts/measure-lsp-channel-handoff.py to also measure process CPU.
use crossbeam_channel::{Receiver, Select, bounded};
use serde::Serialize;
use std::{
    error::Error,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
enum Receive {
    Blocking,
    Select,
}

impl Receive {
    fn receive<T>(self, receiver: &Receiver<T>) -> Result<T, crossbeam_channel::RecvError> {
        match self {
            Self::Blocking => receiver.recv(),
            Self::Select => {
                let mut selection = Select::new_biased();
                selection.recv(receiver);
                selection.select().recv(receiver)
            }
        }
    }
}

struct Request {
    serial: usize,
    sent: Instant,
}

struct Reply {
    serial: usize,
    sent: Instant,
    dispatch_ms: f64,
}

#[derive(Serialize)]
struct Sample {
    serial: usize,
    dispatch_ms: f64,
    reply_ms: f64,
    roundtrip_ms: f64,
    input_send_ms: f64,
    output_send_ms: f64,
}

#[derive(Serialize)]
struct Report {
    mode: String,
    capacity: usize,
    work_us: u64,
    pause_us: u64,
    warmup: usize,
    elapsed_ms: f64,
    samples: Vec<Sample>,
}

fn milliseconds(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

#[allow(
    clippy::disallowed_methods,
    reason = "benchmark instrumentation measures native scheduling outside deterministic language code"
)]
fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [mode, capacity, work_us, pause_us, samples, warmup] = args.as_slice() else {
        return Err("expected MODE CAPACITY WORK_US PAUSE_US SAMPLES WARMUP".into());
    };
    let receive = match mode.as_str() {
        "blocking" => Receive::Blocking,
        "select" => Receive::Select,
        _ => return Err("mode must be blocking or select".into()),
    };
    let capacity = capacity.parse::<usize>()?;
    let work_us = work_us.parse::<u64>()?;
    let pause_us = pause_us.parse::<u64>()?;
    let samples = samples.parse::<usize>()?;
    let warmup = warmup.parse::<usize>()?;
    if ![0, 1].contains(&capacity)
        || samples == 0
        || samples > 10_000
        || warmup > 1_000
        || work_us > 10_000
        || pause_us > 10_000
    {
        return Err("probe settings exceed bounded experiment limits".into());
    }
    let (inputs, worker_inputs) = bounded::<Request>(capacity);
    let (worker_outputs, outputs) = bounded::<Reply>(capacity);
    let started = Instant::now();
    let worker = thread::spawn(move || -> Result<Vec<f64>, String> {
        let mut send_times = Vec::with_capacity(samples + warmup);
        while let Ok(request) = receive.receive(&worker_inputs) {
            let dispatch_ms = milliseconds(request.sent.elapsed());
            let work_started = Instant::now();
            while work_started.elapsed() < Duration::from_micros(work_us) {
                std::hint::spin_loop();
            }
            let sent = Instant::now();
            worker_outputs
                .send(Reply {
                    serial: request.serial,
                    sent,
                    dispatch_ms,
                })
                .map_err(|error| error.to_string())?;
            send_times.push(milliseconds(sent.elapsed()));
        }
        Ok(send_times)
    });
    let mut measured = Vec::with_capacity(samples);
    for serial in 0..samples + warmup {
        thread::sleep(Duration::from_micros(pause_us));
        let sent = Instant::now();
        inputs.send(Request { serial, sent })?;
        let input_send_ms = milliseconds(sent.elapsed());
        // The production coordinator already receives results through select.
        let reply = Receive::Select.receive(&outputs)?;
        let received = Instant::now();
        if reply.serial != serial {
            return Err("channel reply sequence differs from request".into());
        }
        if serial >= warmup {
            measured.push(Sample {
                serial,
                dispatch_ms: reply.dispatch_ms,
                reply_ms: milliseconds(received.duration_since(reply.sent)),
                roundtrip_ms: milliseconds(received.duration_since(sent)),
                input_send_ms,
                output_send_ms: 0.0,
            });
        }
    }
    drop(inputs);
    let send_times = worker.join().map_err(|_| "worker panicked")??;
    if send_times.len() != samples + warmup {
        return Err("worker did not complete every handoff".into());
    }
    for sample in &mut measured {
        sample.output_send_ms = send_times[sample.serial];
    }
    let report = Report {
        mode: mode.clone(),
        capacity,
        work_us,
        pause_us,
        warmup,
        elapsed_ms: milliseconds(started.elapsed()),
        samples: measured,
    };
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer(&mut stdout, &report)?;
    writeln!(stdout)?;
    stdout.flush()?;
    // Keep the child alive so the driver can sample completed process CPU,
    // rather than estimating it from periodic observations before child exit.
    let mut acknowledgement = String::new();
    io::stdin().read_line(&mut acknowledgement)?;
    Ok(())
}
