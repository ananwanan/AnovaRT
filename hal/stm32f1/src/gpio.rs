// SPDX-License-Identifier: Apache-2.0

#[cfg(target_os = "none")]
use crate::mmio;

#[cfg(target_os = "none")]
const RCC_APB2ENR: usize = 0x4002_1018;

#[cfg(target_os = "none")]
#[derive(Clone, Copy)]
pub enum Port {
    B,
    E,
}

#[cfg(target_os = "none")]
impl Port {
    fn registers(self) -> (usize, u32) {
        match self {
            Self::B => (0x4001_0c00, 1 << 3),
            Self::E => (0x4001_1800, 1 << 6),
        }
    }
}

const fn config_field(pin: u8) -> (usize, u32) {
    (if pin < 8 { 0 } else { 4 }, (pin as u32 % 8) * 4)
}

const fn bsrr(pin: u8, high: bool) -> u32 {
    1 << (pin as u32 + if high { 0 } else { 16 })
}

/// A configured STM32F103 GPIO output.
#[cfg(target_os = "none")]
pub struct Output {
    base: usize,
    pin: u8,
}

#[cfg(target_os = "none")]
impl Output {
    /// Enable the port clock and configure a 2 MHz push-pull output.
    ///
    /// # Safety
    ///
    /// Call only on STM32F103 hardware, with a pin verified for output on the board.
    /// The caller must own the pin and serialize RCC and GPIO configuration writes.
    /// Keep the port clock enabled and the pin configured while this value is used.
    pub unsafe fn new(port: Port, pin: u8, initial_high: bool) -> Self {
        assert!(pin < 16);
        let (base, clock) = port.registers();
        let (offset, shift) = config_field(pin);
        // SAFETY: These aligned registers are defined by RM0008. The caller
        // serializes the read-modify-writes. Read back the clock enable before
        // accessing GPIO, and preload ODR via BSRR before enabling the output.
        unsafe {
            mmio::write(RCC_APB2ENR, mmio::read(RCC_APB2ENR) | clock);
            let _ = mmio::read(RCC_APB2ENR);
            mmio::write(base + 0x10, bsrr(pin, initial_high));
            let config = mmio::read(base + offset);
            mmio::write(
                base + offset,
                (config & !(0xf << shift)) | (0b0010 << shift),
            );
        }
        Self { base, pin }
    }

    pub fn set_high(&mut self, high: bool) {
        // SAFETY: Construction establishes pin ownership and GPIO clock state.
        // A single BSRR write changes only this pin's output latch.
        unsafe { mmio::write(self.base + 0x10, bsrr(self.pin, high)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_low_and_high_configuration_words() {
        assert_eq!(config_field(0), (0, 0));
        assert_eq!(config_field(7), (0, 28));
        assert_eq!(config_field(8), (4, 0));
        assert_eq!(config_field(15), (4, 28));
    }

    #[test]
    fn sets_or_resets_only_the_requested_pin() {
        assert_eq!(bsrr(0, true), 0x0000_0001);
        assert_eq!(bsrr(15, true), 0x0000_8000);
        assert_eq!(bsrr(0, false), 0x0001_0000);
        assert_eq!(bsrr(15, false), 0x8000_0000);
    }
}
