// SPDX-License-Identifier: Apache-2.0
//! Cortex-M3 NVIC for integrations with four implemented priority bits.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Priority(u8);

impl Priority {
    pub const fn new(level: u8) -> Option<Self> {
        if level < 16 { Some(Self(level)) } else { None }
    }

    pub const fn level(self) -> u8 {
        self.0
    }
    #[cfg(any(target_os = "none", test))]
    pub(crate) const fn encoded(self) -> u8 {
        self.0 << 4
    }
    #[cfg(any(target_os = "none", test))]
    fn decode(raw: u8) -> Self {
        Self(raw >> 4)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Irq<const COUNT: u8>(u8);

impl<const COUNT: u8> Irq<COUNT> {
    pub const fn new(number: u8) -> Option<Self> {
        if number < COUNT && number < 240 {
            Some(Self(number))
        } else {
            None
        }
    }

    pub const fn number(self) -> u8 {
        self.0
    }

    #[cfg(any(target_os = "none", test))]
    fn bank(self) -> (usize, u32) {
        (usize::from(self.0 / 32) * 4, 1 << (self.0 % 32))
    }
}

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum SystemException {
    MemManage = 4,
    BusFault = 5,
    UsageFault = 6,
    SvCall = 11,
    DebugMonitor = 12,
    PendSv = 14,
    SysTick = 15,
}

#[cfg(target_os = "none")]
mod hardware {
    use super::*;
    use crate::{interrupt, registers as reg};

    impl<const COUNT: u8> Irq<COUNT> {
        fn command(self, base: usize) {
            let (offset, mask) = self.bank();
            reg::write(base + offset, mask);
            interrupt::synchronize();
        }

        fn read_bit(self, base: usize) -> bool {
            let (offset, mask) = self.bank();
            reg::read(base + offset) & mask != 0
        }

        pub fn enable(self) {
            self.command(0xe000_e100);
        }
        pub fn disable(self) {
            self.command(0xe000_e180);
        }
        pub fn set_pending(self) {
            self.command(0xe000_e200);
        }
        pub fn clear_pending(self) {
            self.command(0xe000_e280);
        }
        pub fn is_enabled(self) -> bool {
            self.read_bit(0xe000_e100)
        }
        pub fn is_pending(self) -> bool {
            self.read_bit(0xe000_e200)
        }
        pub fn is_active(self) -> bool {
            self.read_bit(0xe000_e300)
        }
        pub fn set_priority(self, priority: Priority) {
            reg::write_byte(0xe000_e400 + usize::from(self.0), priority.encoded());
            interrupt::synchronize();
        }
        pub fn priority(self) -> Priority {
            Priority::decode(reg::read_byte(0xe000_e400 + usize::from(self.0)))
        }
    }

    impl SystemException {
        pub fn set_priority(self, priority: Priority) {
            reg::write_byte(0xe000_ed18 + self as usize - 4, priority.encoded());
            interrupt::synchronize();
        }
        pub fn priority(self) -> Priority {
            Priority::decode(reg::read_byte(0xe000_ed18 + self as usize - 4))
        }
    }

    /// Select four preemption bits and zero subpriority bits. Call at startup.
    pub fn configure_preemption() {
        // VECTKEY is required. Write no reset request or debug action bits.
        reg::write(0xe000_ed0c, (0x05fa << 16) | (3 << 8));
        interrupt::synchronize();
    }

    pub fn priority_grouping() -> u8 {
        ((reg::read(0xe000_ed0c) >> 8) & 7) as u8
    }

    pub fn pend_pendsv() {
        reg::write(0xe000_ed04, 1 << 28);
        interrupt::synchronize();
    }
}

#[cfg(target_os = "none")]
pub use hardware::{configure_preemption, pend_pendsv, priority_grouping};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq_bounds_and_word_masks() {
        type ChipIrq = Irq<60>;
        for (irq, bank) in [
            (0, (0, 1)),
            (31, (0, 1 << 31)),
            (32, (4, 1)),
            (59, (4, 1 << 27)),
        ] {
            assert_eq!(ChipIrq::new(irq).unwrap().bank(), bank);
        }
        assert_eq!(ChipIrq::new(60), None);
        assert_eq!(Irq::<255>::new(240), None);
    }

    #[test]
    fn priority_encoding() {
        for level in 0..16 {
            let p = Priority::new(level).unwrap();
            assert_eq!(p.encoded(), level << 4);
            assert_eq!(Priority::decode(p.encoded() | 0xf), p);
        }
        assert_eq!(Priority::new(16), None);
        assert_eq!(Priority::new(255), None);
    }
}
