use super::*;
use std::{sync::Barrier, time::Duration};

fn wait_for(mut ready: impl FnMut() -> bool) {
    for _ in 0..10_000 {
        if ready() {
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("worker did not reach the expected state");
}
fn request(base: Option<Stamp>, stamp: Stamp, edits: Option<Vec<InputEdit>>) -> Request {
    Request {
        base,
        stamp,
        rope: Rope::from_str("# current"),
        edits,
        rows: 0..1,
        language: None,
        theme: EditorSyntaxTheme::default(),
    }
}
#[test]
fn pending_slot_keeps_one_latest_request_and_overflow_stays_cold_until_claimed() {
    let mailbox = Arc::new(Mailbox::default());
    let gate = Arc::new(Barrier::new(2));
    let shared = Arc::clone(&mailbox);
    let worker_gate = Arc::clone(&gate);
    let (notify, ready) = async_channel::bounded(1);
    let worker = thread::spawn(move || {
        worker_gate.wait();
        run(&shared, notify);
    });
    let background = BackgroundSyntax {
        mailbox,
        worker,
        ready,
    };
    let mut previous = None;
    for n in 0..200 {
        let stamp = Arc::new(());
        let edit = InputEdit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: 0,
            start_position: tree_sitter::Point::new(0, 0),
            old_end_position: tree_sitter::Point::new(0, 0),
            new_end_position: tree_sitter::Point::new(0, 0),
        };
        assert!(background.submit(request(previous, Arc::clone(&stamp), Some(vec![edit]))));
        previous = Some(stamp);
        let slots = background.mailbox.slots.lock().unwrap();
        let pending = slots.pending.as_ref().unwrap();
        assert!(pending.base.is_none());
        assert!(same(&Some(Arc::clone(&pending.stamp)), &previous));
        if n >= MAX_EDITS {
            assert!(pending.edits.is_none());
        } else {
            assert!(pending.edits.as_ref().unwrap().len() <= MAX_EDITS);
        }
    }
    gate.wait();
    let mut result = None;
    wait_for(|| {
        if result.is_none() {
            result = background.take();
        }
        result.is_some()
    });
    let output = result.unwrap().unwrap();
    assert!(same(&Some(output.stamp), &previous));
    assert_eq!(output.blocks.len(), 1);
}
#[test]
fn shutdown_wakes_sleeping_worker_and_cancels_inflight_parsing() {
    for parsing in [false, true] {
        let background = BackgroundSyntax::new().unwrap();
        if parsing {
            let mut input = request(None, Arc::new(()), None);
            input.rope = Rope::from_str(&":: block\n  Long source 🦊.\n".repeat(100_000));
            input.language = Some(EditorLanguage::new(
                recite_writer_grammar::LANGUAGE,
                recite_writer_grammar::HIGHLIGHTS,
            ));
            assert!(background.submit(input));
            wait_for(|| background.mailbox.slots.lock().unwrap().pending.is_none());
        }
        let weak = Arc::downgrade(&background.mailbox);
        drop(background);
        wait_for(|| weak.upgrade().is_none());
    }
}
#[test]
fn a_finished_worker_rejects_requests_and_reports_failure_instead_of_pending_forever() {
    let background = BackgroundSyntax {
        mailbox: Arc::new(Mailbox::default()),
        worker: thread::spawn(|| {}),
        ready: async_channel::bounded::<()>(1).1,
    };
    wait_for(|| background.worker.is_finished());
    assert!(!background.submit(request(None, Arc::new(()), None)));
    assert!(matches!(background.take(), Some(Err(()))));
}

#[test]
fn completion_notifications_coalesce_and_survive_late_attachment() {
    let background = BackgroundSyntax::new().unwrap();
    let mut previous = None;
    for _ in 0..20 {
        let stamp = Arc::new(());
        assert!(background.submit(request(previous, Arc::clone(&stamp), Some(Vec::new()))));
        wait_for(|| {
            background
                .mailbox
                .slots
                .lock()
                .unwrap()
                .ready
                .as_ref()
                .is_some_and(|output| Arc::ptr_eq(&output.stamp, &stamp))
        });
        previous = Some(stamp);
    }
    let receiver = background.notifications();
    wait_for(|| receiver.len() == 1);
    assert_eq!(receiver.capacity(), Some(1));
    assert!(receiver.try_recv().is_ok());
    assert!(receiver.try_recv().is_err());
    assert!(same(
        &Some(background.take().unwrap().unwrap().stamp),
        &previous
    ));
    background.close();
    wait_for(|| receiver.is_closed());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn viewport_work_does_not_cancel_parsing_or_clear_text_cancellation() {
    let mailbox = Arc::new(Mailbox::default());
    let gate = Arc::new(Barrier::new(2));
    let shared = Arc::clone(&mailbox);
    let worker_gate = Arc::clone(&gate);
    let (notify, ready) = async_channel::bounded(1);
    let worker = thread::spawn(move || {
        worker_gate.wait();
        run(&shared, notify);
    });
    let background = BackgroundSyntax {
        mailbox,
        worker,
        ready,
    };
    let rope = Rope::from_str(":: current\n  Text.\n");
    let language = Some(EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    ));
    background.mailbox.slots.lock().unwrap().active = Some(Active {
        rope: rope.clone(),
        language: language.clone(),
    });
    let viewport = || Request {
        rope: rope.clone(),
        language: language.clone(),
        rows: 1..2,
        ..request(None, Arc::new(()), Some(Vec::new()))
    };
    assert!(background.submit(viewport()));
    assert!(!background.mailbox.superseded.load(Ordering::Relaxed));
    let mut changed = viewport();
    changed.rope.insert(0, "# changed\n");
    assert!(background.submit(changed));
    assert!(background.mailbox.superseded.load(Ordering::Relaxed));
    assert!(background.submit(viewport()));
    assert!(background.mailbox.superseded.load(Ordering::Relaxed));
    background
        .mailbox
        .superseded
        .store(false, Ordering::Relaxed);
    let mut changed = viewport();
    changed.language = None;
    assert!(background.submit(changed));
    assert!(background.mailbox.superseded.load(Ordering::Relaxed));
    background.close();
    gate.wait();
    wait_for(|| background.worker.is_finished());
}
