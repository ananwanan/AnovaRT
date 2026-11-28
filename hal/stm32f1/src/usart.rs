// SPDX-License-Identifier: Apache-2.0
//! Polled USART with STM32F103 fixed 16x oversampling.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidBaud,
    Timeout,
    Receive(u8),
}

/// Encode the nearest representable divider in USART_BRR.
pub fn baud_divider(pclk_hz: u32, baud: u32) -> Result<u16, Error> {
    if baud == 0 {
        return Err(Error::InvalidBaud);
    }
    let divisor = (u64::from(pclk_hz) + u64::from(baud) / 2) / u64::from(baud);
    if !(16..=65_535).contains(&divisor) {
        return Err(Error::InvalidBaud);
    }
    Ok(divisor as u16)
}

#[cfg(target_os = "none")]
mod hardware {
    use super::*;
    use crate::{
        mmio,
        rcc::{Clocks, Peripheral, Rcc},
    };
    use core::fmt;

    const USART1: usize = 0x4001_3800;
    const SR: usize = USART1;
    const DR: usize = USART1 + 4;
    const BRR: usize = USART1 + 8;
    const CR1: usize = USART1 + 0x0c;
    const RXNE: u32 = 1 << 5;
    const TC: u32 = 1 << 6;
    const TXE: u32 = 1 << 7;
    const RX_ERRORS: u32 = 0xf;
    const POLL_LIMIT: u32 = 50_000;

    /// The USART1 peripheral and its default PA9/PA10 pins.
    pub struct Usart1(());

    impl Usart1 {
        /// Initialize USART1 from reset with 8N1 and no flow control.
        ///
        /// # Safety
        /// Call once on STM32F103 after reset with the default USART1 mapping.
        /// The board must verify PA9/PA10 routing and own those pins and USART1.
        /// Keep the clock gates and PCLK2 stable for the lifetime of this driver.
        pub unsafe fn init(rcc: &mut Rcc, clocks: Clocks, baud: u32) -> Result<Self, Error> {
            let divider = baud_divider(clocks.pclk2_hz, baud)?;
            rcc.enable(Peripheral::GpioA);
            rcc.enable(Peripheral::Usart1);
            const GPIOA_CRH: usize = 0x4001_0804;
            const GPIOA_BSRR: usize = 0x4001_0810;
            // SAFETY: Caller owns both pins and the peripheral. Set the idle
            // latch and drive TX only after USART has been enabled.
            unsafe {
                mmio::write(BRR, u32::from(divider));
                mmio::write(CR1, (1 << 13) | (1 << 3) | (1 << 2));
                mmio::write(GPIOA_BSRR, 1 << 9);
                let config = mmio::read(GPIOA_CRH);
                mmio::write(GPIOA_CRH, (config & !(0xff << 4)) | (0x4a << 4));
            }
            Ok(Self(()))
        }

        /// Borrow the initialized peripheral for terminal diagnostic output.
        ///
        /// # Safety
        /// USART1 must already be initialized. Execution must never resume
        /// any other code that uses it, and no interrupt may use it.
        pub unsafe fn for_terminal_diagnostics() -> Self {
            Self(())
        }

        fn wait(&mut self, flag: u32) -> Result<(), Error> {
            // SAFETY: The driver owns the clocked USART and SR is readable.
            if (0..POLL_LIMIT).any(|_| unsafe { mmio::read(SR) } & flag != 0) {
                Ok(())
            } else {
                Err(Error::Timeout)
            }
        }

        pub fn send(&mut self, byte: u8) -> Result<(), Error> {
            self.wait(TXE)?;
            // SAFETY: USART is clocked, TXE is set, and only data bits are written.
            unsafe { mmio::write(DR, u32::from(byte)) };
            Ok(())
        }

        pub fn flush(&mut self) -> Result<(), Error> {
            self.wait(TC)
        }

        pub fn receive(&mut self) -> Result<Option<u8>, Error> {
            // SAFETY: For RXNE/error, RM0008 specifies SR followed by DR.
            let status = unsafe { mmio::read(SR) };
            if status & (RXNE | RX_ERRORS) == 0 {
                return Ok(None);
            }
            // SAFETY: This completes the receive/error-clear sequence.
            let data = unsafe { mmio::read(DR) } as u8;
            if status & RX_ERRORS != 0 {
                return Err(Error::Receive((status & RX_ERRORS) as u8));
            }
            Ok(Some(data))
        }
    }

    impl fmt::Write for Usart1 {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            for byte in text.bytes() {
                self.send(byte).map_err(|_| fmt::Error)?;
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "none")]
pub use hardware::Usart1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_divider_uses_pclk2_and_rounds() {
        assert_eq!(baud_divider(72_000_000, 115_200), Ok(625));
        assert_eq!(baud_divider(8_000_000, 115_200), Ok(69));
        assert_eq!(baud_divider(8_000_000, 0), Err(Error::InvalidBaud));
        assert_eq!(baud_divider(8_000_000, 1), Err(Error::InvalidBaud));
        assert_eq!(baud_divider(8_000_000, 1_000_000), Err(Error::InvalidBaud));
    }
}
