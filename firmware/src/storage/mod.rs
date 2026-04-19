/// Storage module for persistent data.

pub mod flash;
#[cfg(feature = "sdcard")]
pub mod sdcard;
