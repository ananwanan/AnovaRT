// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(target_os = "none")]
use anova_stm32f1::gpio::{Output, Port};

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
