// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

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

    #[cfg(feature = "lcd-cat")]
    {
        // SAFETY: The operator explicitly selects this experiment only after
        // verifying the wiring. Reset masks interrupts; the demo owns MMIO.
        unsafe {
            anova_board::lcd_cat::run(|ms| {
                for _ in 0..ms {
                    anova_cortex_m::delay_iterations(72_000);
                }
            });
        }
        anova_cortex_m::halt();
    }

    #[cfg(not(feature = "lcd-cat"))]
    {
        // SAFETY: This entry runs once on the Elite ME V6 with interrupts masked.
        // It is the sole owner of PB5 and the only writer to RCC/GPIOB.
        let mut led = unsafe { anova_board::Led::init() };
        loop {
            led.set(true);
            anova_cortex_m::delay_iterations(1_000_000);
            led.set(false);
            anova_cortex_m::delay_iterations(1_000_000);
        }
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    anova_cortex_m::halt()
}

#[cfg(not(target_os = "none"))]
fn main() {
    eprintln!("Build the firmware for thumbv7m-none-eabi with cargo m0-build.");
}
