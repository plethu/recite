use super::*;
use std::{fs, sync::mpsc as sync_mpsc, time::Duration};
#[test]
fn idle_watch_posts_no_ui_work_and_teardown_cancels_debounce_children() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.recite");
    fs::write(&path, "original").unwrap();
    let watch = Watch::new(directory.path()).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (send, output) = sync_mpsc::channel();
    let (_send, requests) = mpsc::channel(1);
    let (_scope, binding) = watch::channel(None);
    let service = runtime.spawn(serve(
        move |output| send.send(output).is_ok(),
        WindowKey::next(),
        Some(Ok(watch.subscribe())),
        requests,
        binding,
    ));
    assert!(matches!(
        output.recv_timeout(Duration::from_millis(150)),
        Err(sync_mpsc::RecvTimeoutError::Timeout)
    ));
    fs::write(&path, "changed").unwrap();
    let message = output.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(message.event, Event::Sources(changes) if changes.affected.contains(&path)));
    service.abort();
    assert!(runtime.block_on(service).unwrap_err().is_cancelled());
    runtime.block_on(tokio::task::yield_now());
    let _ = watch.changed();
    fs::write(&path, "after teardown").unwrap();
    std::thread::sleep(Duration::from_millis(650));
    assert!(
        watch.changed().affected.contains(&path),
        "the retired child must not consume future notifications"
    );
    assert!(output.try_recv().is_err());
}
