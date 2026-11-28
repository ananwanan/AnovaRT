// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(target_os = "none")]
core::arch::global_asm!(
    include_str!("startup.s"),
    m4 = const cfg!(feature = "m4") as u8,
    m3 = const cfg!(feature = "m3") as u8,
    m2 = const cfg!(feature = "m2") as u8,
);

#[cfg(all(target_os = "none", feature = "m3"))]
pub mod stack;

#[cfg(all(target_os = "none", feature = "m3"))]
pub mod cycles;

pub mod fault;
pub mod nvic;
pub mod systick;

#[cfg(target_os = "none")]
pub mod interrupt;

#[cfg(target_os = "none")]
mod registers {
    use core::ptr::{read_volatile, write_volatile};

    // These helpers are private to the architecture layer. Every caller
    // supplies an aligned, privileged System Control Space register address.
    pub(crate) fn read(address: usize) -> u32 {
        // SAFETY: Callers select a readable, word-aligned SCS register.
        unsafe { read_volatile(address as *const u32) }
    }

    pub(crate) fn write(address: usize, value: u32) {
        // SAFETY: Callers select a writable word register and valid write bits.
        unsafe { write_volatile(address as *mut u32, value) }
    }

    pub(crate) fn read_byte(address: usize) -> u8 {
        // SAFETY: Callers select a byte-addressable NVIC/SCB priority field.
        unsafe { read_volatile(address as *const u8) }
    }

    pub(crate) fn write_byte(address: usize, value: u8) {
        // SAFETY: Callers select a writable byte priority field and encoding.
        unsafe { write_volatile(address as *mut u8, value) }
    }
}

/// Busy-wait for a bounded number of iterations at the current core clock.
#[cfg(target_os = "none")]
pub fn delay_iterations(count: u32) {
    if count == 0 {
        return;
    }
    // SAFETY: The loop uses only its register operand and condition flags.
    unsafe {
        core::arch::asm!(
            "2: subs {count}, #1",
            "bne 2b",
            count = inout(reg) count => _,
            options(nomem, nostack)
        );
    }
}

/// Park the core with configurable exceptions masked.
#[cfg(target_os = "none")]
pub fn halt() -> ! {
    // SAFETY: M0 runs privileged and this terminal path never returns.
    unsafe {
        core::arch::asm!(
            "cpsid i",
            "2: wfi",
            "b 2b",
            options(noreturn, nomem, nostack)
        )
    }
}

pub mod context;
