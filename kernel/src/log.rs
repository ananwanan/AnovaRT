// SPDX-License-Identifier: Apache-2.0
//! Allocation-free synchronous logging with a caller-owned sink.

use core::fmt::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    const fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

pub const MAX_VERBOSITY: Level = if cfg!(feature = "log-trace") {
    Level::Trace
} else if cfg!(feature = "log-debug") {
    Level::Debug
} else {
    Level::Info
};

pub const fn enabled(level: Level) -> bool {
    level as u8 >= MAX_VERBOSITY as u8
}

/// Implementations can supply a timestamp and serialize complete records.
/// M1 uses a single mutable writer and supplies no timestamp.
pub trait Sink {
    fn write_record(&mut self, level: Level, message: fmt::Arguments<'_>) -> fmt::Result;
}

pub struct TextSink<'a, W: Write>(pub &'a mut W);

impl<W: Write> Sink for TextSink<'_, W> {
    fn write_record(&mut self, level: Level, message: fmt::Arguments<'_>) -> fmt::Result {
        write!(self.0, "[{}] {}\r\n", level.label(), message)
    }
}

#[doc(hidden)]
pub fn emit(sink: &mut impl Sink, level: Level, message: fmt::Arguments<'_>) -> fmt::Result {
    sink.write_record(level, message)
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log {
    ($sink:expr, $level:ident, $($arg:tt)+) => {{
        const LEVEL: $crate::log::Level = $crate::log::Level::$level;
        if $crate::log::enabled(LEVEL) {
            $crate::log::emit($sink, LEVEL, core::format_args!($($arg)+))
        } else {
            Ok(())
        }
    }};
}

#[macro_export]
macro_rules! trace {
    ($sink:expr, $($arg:tt)+) => { $crate::__log!($sink, Trace, $($arg)+) };
}

#[macro_export]
macro_rules! debug {
    ($sink:expr, $($arg:tt)+) => { $crate::__log!($sink, Debug, $($arg)+) };
}

#[macro_export]
macro_rules! info {
    ($sink:expr, $($arg:tt)+) => { $crate::__log!($sink, Info, $($arg)+) };
}

#[macro_export]
macro_rules! warn {
    ($sink:expr, $($arg:tt)+) => { $crate::__log!($sink, Warn, $($arg)+) };
}

#[macro_export]
macro_rules! error {
    ($sink:expr, $($arg:tt)+) => { $crate::__log!($sink, Error, $($arg)+) };
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::String;

    #[test]
    fn filters_before_evaluating_arguments() {
        let mut text = String::new();
        let mut sink = TextSink(&mut text);
        let mut trace_calls = 0;
        let mut debug_calls = 0;
        trace!(&mut sink, "{}", {
            trace_calls += 1;
            1
        })
        .unwrap();
        debug!(&mut sink, "{}", {
            debug_calls += 1;
            2
        })
        .unwrap();
        info!(&mut sink, "{}", 3).unwrap();
        warn!(&mut sink, "{}", 4).unwrap();
        error!(&mut sink, "{}", 5).unwrap();
        assert_eq!(trace_calls, u8::from(cfg!(feature = "log-trace")));
        assert_eq!(
            debug_calls,
            u8::from(cfg!(any(feature = "log-trace", feature = "log-debug")))
        );
        let expected = if cfg!(feature = "log-trace") {
            "[TRACE] 1\r\n[DEBUG] 2\r\n"
        } else if cfg!(feature = "log-debug") {
            "[DEBUG] 2\r\n"
        } else {
            ""
        };
        assert_eq!(
            text.strip_prefix(expected),
            Some("[INFO] 3\r\n[WARN] 4\r\n[ERROR] 5\r\n")
        );
    }
}
