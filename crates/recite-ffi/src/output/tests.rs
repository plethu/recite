use std::error::Error as _;
use std::io::{self, Write};

use recite_runtime::DialogueEvent;

use super::{encode_batch, encode_batch_to_writer};
use crate::{FfiOutputEncodeError, ReciteStatus, encode_batch_output};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn output_encoder_retains_messagepack_source_error() {
    let mut writer = FailingWriter;
    let error = encode_batch_to_writer(Vec::new(), &mut writer)
        .expect_err("the injected writer failure must reach the encoder");

    let source = error.source().expect("typed encoder source is retained");
    assert!(
        source.downcast_ref::<rmp_serde::encode::Error>().is_some(),
        "source should remain the rmp-serde encode error"
    );
    assert!(
        error
            .to_string()
            .contains("failed to encode FFI output batch")
    );

    let Err((status, message)) = encode_batch_output(Vec::new(), failing_encode) else {
        panic!("the production output seam must map encoder failures");
    };
    assert_eq!(status, ReciteStatus::DialogueFault);
    assert!(message.contains("failed to encode FFI output batch"));
}

fn failing_encode(events: Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError> {
    let mut writer = FailingWriter;
    encode_batch_to_writer(events, &mut writer).map(|()| Vec::new())
}

#[test]
fn empty_and_normal_batches_use_the_same_messagepack_encoder() {
    let empty = encode_batch(Vec::new()).expect("empty batch encodes");
    let normal = encode_batch(vec![DialogueEvent::End {
        deferred_effects: Vec::new(),
    }])
    .expect("normal batch encodes");

    let empty: serde_json::Value = rmp_serde::from_slice(&empty).expect("empty batch decodes");
    let normal: serde_json::Value = rmp_serde::from_slice(&normal).expect("normal batch decodes");
    assert_eq!(empty["batch_format_version"], 0);
    assert_eq!(normal["batch_format_version"], 0);
    assert_eq!(empty["events"].as_array().expect("events array").len(), 0);
    assert_eq!(normal["events"][0]["kind"], "end");
}
