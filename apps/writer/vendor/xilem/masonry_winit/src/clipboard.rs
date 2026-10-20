// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use copypasta::{ClipboardContext, ClipboardProvider};
use winit::event_loop::OwnedDisplayHandle;
use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn connect(display: OwnedDisplayHandle) -> Result<Box<dyn ClipboardProvider>, Error> {
    match display.display_handle()?.as_raw() {
        RawDisplayHandle::Wayland(_) => {
            let (_, clipboard) =
                copypasta::wayland_clipboard::create_clipboards_from_display(display)?;
            Ok(Box::new(clipboard))
        }
        _ => ClipboardContext::new()
            .map(|clipboard| -> Box<dyn ClipboardProvider> { Box::new(clipboard) }),
    }
}
