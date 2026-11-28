// SPDX-License-Identifier: Apache-2.0
//! An allocation-free wrapping count of delivered one-millisecond ticks.

use core::sync::atomic::{AtomicU32, Ordering};

pub const TICK_HZ: u32 = 1_000;

/// One writer advances the clock. Readers can sample from any context.
pub struct Timebase(AtomicU32);

impl Timebase {
    pub const fn new(initial_ticks: u32) -> Self {
        Self(AtomicU32::new(initial_ticks))
    }

    pub fn tick(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn ticks(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }

    /// Wraps every 2^32 ticks. Masked ticks can coalesce in the hardware.
    pub fn uptime_ms(&self) -> u32 {
        self.ticks()
    }

    /// Valid when fewer than 2^32 ticks have elapsed since the sample.
    pub fn elapsed_since(&self, start: u32) -> u32 {
        self.ticks().wrapping_sub(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_and_elapsed() {
        let clock = Timebase::new(u32::MAX);
        assert_eq!(clock.uptime_ms(), u32::MAX);
        clock.tick();
        assert_eq!(clock.ticks(), 0);
        clock.tick();
        assert_eq!(clock.elapsed_since(u32::MAX), 2);
    }
}
