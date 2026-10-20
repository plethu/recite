#![cfg(all(
    unix,
    not(any(
        target_os = "macos",
        target_os = "android",
        target_os = "ios",
        target_os = "emscripten"
    )),
    feature = "wayland"
))]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use copypasta::wayland_clipboard::create_clipboards_from_display;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};

struct TrackedDisplay<D> {
    inner: D,
    drops: Arc<AtomicUsize>,
}

impl<D: HasDisplayHandle> HasDisplayHandle for TrackedDisplay<D> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

impl<D> Drop for TrackedDisplay<D> {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

enum RejectedDisplay {
    Unavailable,
    OtherPlatform,
}

impl HasDisplayHandle for RejectedDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        match self {
            Self::Unavailable => Err(HandleError::Unavailable),
            Self::OtherPlatform => Ok(DisplayHandle::appkit()),
        }
    }
}

#[test]
fn unavailable_display_releases_owner() {
    let drops = Arc::new(AtomicUsize::new(0));
    let display = TrackedDisplay { inner: RejectedDisplay::Unavailable, drops: drops.clone() };
    assert!(matches!(create_clipboards_from_display(display), Err(HandleError::Unavailable)));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn unsupported_display_releases_owner() {
    let drops = Arc::new(AtomicUsize::new(0));
    let display = TrackedDisplay { inner: RejectedDisplay::OtherPlatform, drops: drops.clone() };
    assert!(matches!(create_clipboards_from_display(display), Err(HandleError::NotSupported)));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an owned private Wayland compositor"]
fn native_display_lives_until_last_clipboard_handle() -> Result<(), Box<dyn std::error::Error>> {
    use winit::event_loop::EventLoop;
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    let event_loop = EventLoop::builder().with_wayland().with_any_thread(true).build()?;
    let drops = Arc::new(AtomicUsize::new(0));
    let (primary_a, clipboard_a) = create_clipboards_from_display(TrackedDisplay {
        inner: event_loop.owned_display_handle(),
        drops: drops.clone(),
    })?;
    let (primary_b, clipboard_b) = create_clipboards_from_display(TrackedDisplay {
        inner: event_loop.owned_display_handle(),
        drops: drops.clone(),
    })?;
    drop(event_loop);
    drop(primary_a);
    drop(clipboard_b);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(clipboard_a);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(primary_b);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    Ok(())
}
