/// Shared formatting utilities for `heapless::String`.
///
/// Extracted to avoid duplicating the `HeaplessWriter` adapter
/// across `main.rs`, `wifi.rs`, and `mqtt.rs`.

use heapless::String;

/// Adapter that implements `core::fmt::Write` for `heapless::String`,
/// enabling use with `core::fmt::write()` and `write!()` macros.
pub struct HeaplessWriter<'a, const N: usize>(pub &'a mut String<N>);

impl<const N: usize> core::fmt::Write for HeaplessWriter<'_, N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.push_str(s).map_err(|_| core::fmt::Error)
    }
}

/// Format a value into a new `heapless::String`, returning a default on overflow.
///
/// # Example
/// ```ignore
/// let s: heapless::String<16> = format_heapless!("ws-{:04X}", 0x1234);
/// ```
#[macro_export]
macro_rules! format_heapless {
    ($($arg:tt)*) => {{
        let mut s = heapless::String::new();
        let _ = core::fmt::write(
            &mut $crate::utils::fmt::HeaplessWriter(&mut s),
            format_args!($($arg)*),
        );
        s
    }};
}
