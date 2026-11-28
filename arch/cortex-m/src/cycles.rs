// SPDX-License-Identifier: Apache-2.0
//! Optional Cortex-M3 DWT cycle counter.

use crate::{interrupt, registers};

const DEMCR: usize = 0xe000_edfc;
const DWT_CTRL: usize = 0xe000_1000;
const CYCCNT: usize = 0xe000_1004;
const TRACE_ENABLE: u32 = 1 << 24;
const NO_COUNTER: u32 = 1 << 25;

/// A Thread Mode diagnostic session. The firmware must own DWT configuration.
pub struct CycleCounter {
    demcr: u32,
    control: u32,
}

impl CycleCounter {
    pub fn probe() -> Option<Self> {
        let demcr = registers::read(DEMCR);
        registers::write(DEMCR, demcr | TRACE_ENABLE);
        interrupt::synchronize();
        if registers::read(DEMCR) & TRACE_ENABLE == 0 {
            registers::write(DEMCR, demcr);
            return None;
        }
        let counter = Self {
            demcr,
            control: registers::read(DWT_CTRL),
        };
        if counter.control == 0 || counter.control & NO_COUNTER != 0 {
            return None;
        }
        registers::write(DWT_CTRL, counter.control | 1);
        interrupt::synchronize();
        let start = counter.read();
        crate::delay_iterations(32);
        (counter.read() != start).then_some(counter)
    }

    #[inline]
    pub fn read(&self) -> u32 {
        registers::read(CYCCNT)
    }
}

impl Drop for CycleCounter {
    fn drop(&mut self) {
        if self.control != 0 && self.control & NO_COUNTER == 0 {
            registers::write(DWT_CTRL, self.control);
        }
        registers::write(DEMCR, self.demcr);
        interrupt::synchronize();
    }
}
