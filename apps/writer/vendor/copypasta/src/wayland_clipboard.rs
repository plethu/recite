// Copyright 2017 Avraham Weinstock
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use raw_window_handle::{HandleError, HasDisplayHandle, RawDisplayHandle};
use smithay_clipboard::Clipboard as WaylandClipboard;

use crate::common::{ClipboardProvider, Result};

pub struct Clipboard {
    context: Arc<Mutex<Context>>,
}

pub struct Primary {
    context: Arc<Mutex<Context>>,
}

// Drop the worker (which joins its thread) before releasing its display owner.
struct Context {
    clipboard: WaylandClipboard,
    _display: Option<Box<dyn HasDisplayHandle + Send + Sync>>,
}

/// Create clipboards that own a Wayland display handle.
///
/// The display owner is retained until both returned handles are dropped, and the
/// clipboard worker is stopped before the owner is released. Returns
/// [`HandleError::NotSupported`] for a non-Wayland display, or propagates the
/// owner's handle error. Successful construction does not establish protocol
/// availability or keyboard focus; clipboard operations can still fail.
///
/// ```no_run
/// # fn connect(display: impl raw_window_handle::HasDisplayHandle + Send + Sync + 'static)
/// # -> Result<(), raw_window_handle::HandleError> {
/// let (primary, clipboard) =
///     copypasta::wayland_clipboard::create_clipboards_from_display(display)?;
/// # Ok(())
/// # }
/// ```
///
/// A temporary borrow cannot serve as the display owner:
///
/// ```compile_fail,E0521
/// use raw_window_handle::HasDisplayHandle;
/// fn connect<D: HasDisplayHandle + Send + Sync>(display: &D) {
///     let _ = copypasta::wayland_clipboard::create_clipboards_from_display(display);
/// }
/// ```
pub fn create_clipboards_from_display<D>(
    display: D,
) -> std::result::Result<(Primary, Clipboard), HandleError>
where
    D: HasDisplayHandle + Send + Sync + 'static,
{
    // Stabilize the owner's address before borrowing its display handle.
    let display = Box::new(display);
    let RawDisplayHandle::Wayland(handle) = display.display_handle()?.as_raw() else {
        return Err(HandleError::NotSupported);
    };
    // SAFETY: HasDisplayHandle supplies a valid display. The boxed owner is not
    // moved or exposed again and is retained in the shared context until after
    // the worker has stopped, including if either public handle is forgotten.
    let clipboard = unsafe { WaylandClipboard::new(handle.display.as_ptr()) };
    Ok(clipboards(Context { clipboard, _display: Some(display) }))
}

/// Create new clipboards from a raw display pointer.
///
/// # Safety
///
/// `display` must be a valid Wayland display and remain valid until both returned
/// handles have been dropped and the clipboard worker has stopped.
pub unsafe fn create_clipboards_from_external(display: *mut c_void) -> (Primary, Clipboard) {
    // SAFETY: The caller guarantees the display's validity and lifetime.
    let clipboard = unsafe { WaylandClipboard::new(display) };
    clipboards(Context { clipboard, _display: None })
}

fn clipboards(context: Context) -> (Primary, Clipboard) {
    let context = Arc::new(Mutex::new(context));
    (Primary { context: context.clone() }, Clipboard { context })
}

impl ClipboardProvider for Clipboard {
    fn get_contents(&mut self) -> Result<String> {
        Ok(self.context.lock().unwrap().clipboard.load()?)
    }

    fn set_contents(&mut self, data: String) -> Result<()> {
        self.context.lock().unwrap().clipboard.store(data);

        Ok(())
    }
}

impl ClipboardProvider for Primary {
    fn get_contents(&mut self) -> Result<String> {
        Ok(self.context.lock().unwrap().clipboard.load_primary()?)
    }

    fn set_contents(&mut self, data: String) -> Result<()> {
        self.context.lock().unwrap().clipboard.store_primary(data);

        Ok(())
    }
}
