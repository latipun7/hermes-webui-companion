//! Shared constants and debug logging for the GUI binary.

/// Log only when HERMES_COMPANION_DEBUG=1.
macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var("HERMES_COMPANION_DEBUG").unwrap_or_default() == "1" {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use debug;

/// Standard petdex frame aspect ratio: width / height.
pub const ASPECT_RATIO: f64 = 192.0 / 208.0;
