use super::*;
use notify::{
    Event, EventKind,
    event::{AccessKind, ModifyKind},
};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

#[test]
fn an_idle_subscription_waits_without_emitting_work_and_ignores_read_events() {
    let (inbox, source) = channel();
    let mut future = std::pin::pin!(source.next());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    inbox.accept(Ok(
        Event::new(EventKind::Access(AccessKind::Any)).add_path("read.recite".into())
    ));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    inbox.accept(Ok(
        Event::new(EventKind::Modify(ModifyKind::Any)).add_path("changed.recite".into())
    ));
    let Poll::Ready(Some(changes)) = future.as_mut().poll(&mut context) else {
        panic!("actual change")
    };
    assert!(changes.affected.contains(Path::new("changed.recite")));
}

#[test]
fn large_distinct_bursts_coalesce_to_one_wakeup_and_bounded_rescan_state() {
    for count in [64, 10_260] {
        let (inbox, source) = channel();
        for index in 0..count {
            inbox.accept(Ok(Event::new(EventKind::Modify(ModifyKind::Any))
                .add_path(format!("{index}.recite").into())));
        }
        inbox.accept(Err(notify::Error::generic("watcher failure")));
        assert_eq!(source.wake.len(), 1);
        let changes = source.changed();
        assert!(
            changes
                .error
                .as_deref()
                .unwrap()
                .contains("watcher failure")
        );
        match changes.affected {
            Affected::Paths(paths) => assert_eq!(paths.len(), 64),
            Affected::AllOpen => assert!(count > 64),
        }
        assert!(source.changed().affected.is_empty());
    }
}

#[test]
fn repeated_paths_and_kernel_rescan_flags_preserve_the_complete_advisory_scope() {
    let (inbox, source) = channel();
    for _ in 0..1_026 {
        inbox.accept(Ok(
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path("one.recite".into())
        ));
    }
    assert!(matches!(source.changed().affected, Affected::Paths(paths) if paths.len() == 1));
    inbox.accept(Ok(
        Event::new(EventKind::Other).set_flag(notify::event::Flag::Rescan)
    ));
    assert!(matches!(source.changed().affected, Affected::AllOpen));
}
