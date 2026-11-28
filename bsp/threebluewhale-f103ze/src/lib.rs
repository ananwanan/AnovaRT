// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(target_os = "none")]
use anova_stm32f1::gpio::{Output, Port};

#[cfg(target_os = "none")]
pub use anova_stm32f1::{rcc, usart};

pub const NAME: &str = "Three Blue Whales Elite ME V6";
pub const MCU: &str = "STM32F103ZET6 / Cortex-M3";
pub const FLASH_KIB: u32 = 512;
pub const SRAM_KIB: u32 = 64;
pub const CONSOLE_BAUD: u32 = 115_200;

#[cfg(target_os = "none")]
pub fn init_clock(
    rcc: &mut rcc::Rcc,
    fault: rcc::Fault,
) -> Result<rcc::ClockInit, rcc::ClockError> {
    let crystal = if cfg!(feature = "confirmed-hse-8mhz") {
        rcc::ExternalClock::Crystal8Mhz
    } else {
        rcc::ExternalClock::Unconfirmed
    };
    rcc.configure(crystal, fault)
}

#[cfg(target_os = "none")]
/// # Safety
/// Call once after clock setup with exclusive ownership of USART1 and PA9/PA10.
/// Keep the clock configuration and pin assignment fixed afterwards.
pub unsafe fn init_console(
    rcc: &mut rcc::Rcc,
    clocks: rcc::Clocks,
) -> Result<Option<usart::Usart1>, usart::Error> {
    if cfg!(feature = "confirmed-usart1") {
        // SAFETY: This feature is the operator's confirmation of the board
        // routing and that PA9 can be driven. The startup path owns USART1
        // and both pins, and the BSP keeps clocks unchanged after startup.
        unsafe { usart::Usart1::init(rcc, clocks, CONSOLE_BAUD) }.map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "none")]
const DS0_ACTIVE_HIGH: bool = false;

/// The Elite ME V6 DS0 LED on PB5.
#[cfg(target_os = "none")]
pub struct Led(Output);

#[cfg(target_os = "none")]
impl Led {
    /// Configure DS0 in its inactive state.
    ///
    /// # Safety
    ///
    /// Call once on the Elite ME V6 after reset, with exclusive ownership of
    /// PB5 and no concurrent RCC or GPIOB configuration.
    pub unsafe fn init() -> Self {
        // SAFETY: The board binding is PB5, and the caller owns its configuration.
        Self(unsafe { Output::new(Port::B, 5, !DS0_ACTIVE_HIGH) })
    }

    pub fn set(&mut self, on: bool) {
        self.0.set_high(on == DS0_ACTIVE_HIGH);
    }
}
