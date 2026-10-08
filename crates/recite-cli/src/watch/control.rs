use std::io::{self, BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use recite_compiler::authoring::BuildControl;
use serde::Deserialize;

/// A process-scoped control message received on structured watch stdin.
#[derive(Debug)]
pub(super) enum ControlMessage {
    Cancel,
    Error(ControlError),
    Stream(io::Error),
}

/// Recoverable control-input failures. These intentionally carry no parser
/// text: structured clients branch on the stable kind, not a display string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) enum ControlError {
    Malformed,
    UnsupportedVersion,
    UnsupportedCommand,
    UnsupportedAction,
    InvocationMismatch,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlWire {
    version: u16,
    command: String,
    action: String,
    #[serde(default)]
    invocation_id: Option<String>,
}

/// The stdin transport owns only cancellation signalling. The watch host
/// remains the owner of lifecycle state and event ordering.
#[derive(Clone, Debug)]
pub(super) struct ControlTransport {
    active: Arc<Mutex<Option<BuildControl>>>,
    requested: Arc<AtomicBool>,
}

impl ControlTransport {
    pub(super) fn spawn<R>(
        reader: R,
        invocation_id: Option<&str>,
    ) -> (Self, mpsc::Receiver<ControlMessage>)
    where
        R: Read + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let active = Arc::new(Mutex::new(None));
        let requested = Arc::new(AtomicBool::new(false));
        let transport = Self {
            active: Arc::clone(&active),
            requested: Arc::clone(&requested),
        };
        let invocation_id = invocation_id.map(str::to_owned);
        std::thread::spawn(move || {
            let reader = BufReader::new(reader);
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        let _ = sender.send(ControlMessage::Stream(error));
                        return;
                    }
                };
                let message = match serde_json::from_str::<ControlWire>(&line) {
                    Ok(control) => match control.validate(invocation_id.as_deref()) {
                        Err(error) => ControlMessage::Error(error),
                        Ok(()) => {
                            requested.store(true, Ordering::Release);
                            if let Ok(active) = active.lock()
                                && let Some(control) = active.as_ref()
                            {
                                control.cancel();
                            }
                            ControlMessage::Cancel
                        }
                    },
                    Err(_) => ControlMessage::Error(ControlError::Malformed),
                };
                if sender.send(message).is_err() {
                    return;
                }
            }
        });
        (transport, receiver)
    }

    pub(super) fn begin_build(&self, control: &BuildControl) {
        if let Ok(mut active) = self.active.lock() {
            *active = Some(control.clone());
            if self.requested.load(Ordering::Acquire) {
                control.cancel();
            }
        }
    }

    pub(super) fn end_build(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
    }

    pub(super) fn requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

impl ControlWire {
    fn validate(&self, expected_invocation: Option<&str>) -> Result<(), ControlError> {
        if self.version != 1 {
            return Err(ControlError::UnsupportedVersion);
        }
        if self.command != "watch" {
            return Err(ControlError::UnsupportedCommand);
        }
        if self.action != "cancel" {
            return Err(ControlError::UnsupportedAction);
        }
        if !matching_invocation(expected_invocation, self.invocation_id.as_deref()) {
            return Err(ControlError::InvocationMismatch);
        }
        Ok(())
    }
}

fn matching_invocation(expected: Option<&str>, received: Option<&str>) -> bool {
    match (expected, received) {
        (Some(expected), Some(received)) => expected == received,
        (Some(_), None) => true,
        (None, None) => true,
        (None, Some(_)) => false,
    }
}

#[cfg(test)]
mod tests;
