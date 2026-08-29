/// Heap-allocated byte buffer returned by `recite-ffi`.
///
/// The host must call `recite_buffer_free` exactly once after consuming the
/// data. The buffer is allocated by Rust's global allocator; do not free it
/// with a different allocator.
///
/// # Safety
/// Freeing with the wrong allocator (e.g. the Unity C runtime's `free`) is
/// undefined behaviour. Always link against `recite-ffi` as a pre-built
/// `.dll`/`.so`; never recompile it per backend or link it statically into
/// a Unity player separately.
#[repr(C)]
pub struct ReciteBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl ReciteBuffer {
    pub fn null() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }

    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        // Use Box<[u8]> so the allocator sees the exact allocation size on free,
        // avoiding the capacity-vs-len mismatch that Vec::from_raw_parts can cause.
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let data = Box::into_raw(boxed) as *mut u8;
        Self { data, len }
    }
}

/// Frees a buffer allocated by `recite-ffi`.
///
/// # Safety
/// `buf` must have been produced by a `recite-ffi` function and must not have
/// been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_buffer_free(buf: *mut ReciteBuffer) {
    if buf.is_null() {
        return;
    }
    let buf = unsafe { &mut *buf };
    if !buf.data.is_null() && buf.len > 0 {
        // SAFETY: data+len were produced by Box<[u8]>::into_raw in from_bytes.
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf.data, buf.len));
        }
    }
    // Always zero both fields so the host sees a clean null state after free,
    // even when len was 0 and no deallocation ran.
    buf.data = std::ptr::null_mut();
    buf.len = 0;
}
