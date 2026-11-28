// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(all(target_os = "none", feature = "heap-tlsf"))]
extern crate alloc;

#[cfg(all(target_os = "none", feature = "m1"))]
mod m1;
#[cfg(all(target_os = "none", feature = "m2"))]
mod m2;
#[cfg(all(target_os = "none", feature = "m3"))]
mod m3;

#[cfg(target_os = "none")]
static mut STARTUP_DATA: u32 = 0x4d30_5254;
#[cfg(target_os = "none")]
static mut STARTUP_BSS: u32 = 0;

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
extern "C" fn firmware_main() -> ! {
    // SAFETY: Reset prepares these words before entry. No other code in the
    // firmware accesses them. Volatile reads retain the startup self-check.
    let initialized = unsafe {
        core::ptr::read_volatile(&raw const STARTUP_DATA) == 0x4d30_5254
            && core::ptr::read_volatile(&raw const STARTUP_BSS) == 0
    };
    if !initialized {
        anova_cortex_m::halt();
    }

    // SAFETY: This entry runs once on the Elite ME V6 with interrupts masked.
    // It is the sole owner of PB5 and the only writer to RCC/GPIOB.
    let led = unsafe { anova_board::Led::init() };

    #[cfg(feature = "m2")]
    m2::run(led);

    #[cfg(all(feature = "m1", not(feature = "m2")))]
    m1::run(led);

    #[cfg(not(feature = "m1"))]
    {
        let mut led = led;
        loop {
            led.set(true);
            anova_cortex_m::delay_iterations(1_000_000);
            led.set(false);
            anova_cortex_m::delay_iterations(1_000_000);
        }
    }
}

#[cfg(all(target_os = "none", not(feature = "m1")))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    anova_cortex_m::halt()
}

#[cfg(not(target_os = "none"))]
fn main() {
    eprintln!("Build the firmware for thumbv7m-none-eabi with cargo m0-build.");
}

#[cfg(all(target_os = "none", feature = "m4"))]
mod m4;
