//! Bounded framing for the local activation socket.
use std::{
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    time::Instant,
};

use serde::{Deserialize, Serialize};

use super::IO_TIMEOUT;

pub(super) const MAX_FRAME: usize = 64 * 1024;

pub(super) fn read_frame<T: for<'de> Deserialize<'de>>(
    stream: &mut UnixStream,
) -> Result<T, String> {
    let deadline = monotonic_now() + IO_TIMEOUT;
    let mut length = [0u8; 4];
    read_exact_before(stream, &mut length, deadline)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err("Invalid or oversized writer message".into());
    }
    let mut bytes = vec![0u8; length];
    read_exact_before(stream, &mut bytes, deadline)?;
    serde_json::from_slice(&bytes).map_err(|_| "Invalid writer message".into())
}

pub(super) fn write_frame<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FRAME {
        return Err("Oversized writer message".into());
    }
    let deadline = monotonic_now() + IO_TIMEOUT;
    write_all_before(stream, &(bytes.len() as u32).to_be_bytes(), deadline)?;
    write_all_before(stream, &bytes, deadline)
}

fn read_exact_before(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        let remaining = deadline
            .checked_duration_since(monotonic_now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or("Writer message timed out")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|e| e.to_string())?;
        match stream.read(bytes) {
            Ok(0) => return Err("Writer message ended early".into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn write_all_before(
    stream: &mut UnixStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        let remaining = deadline
            .checked_duration_since(monotonic_now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or("Writer message timed out")?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|e| e.to_string())?;
        match stream.write(bytes) {
            Ok(0) => return Err("Writer connection closed while sending".into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

#[allow(
    clippy::disallowed_methods,
    reason = "host activation deadlines stay outside deterministic dialogue state"
)]
pub(super) fn monotonic_now() -> Instant {
    Instant::now()
}
