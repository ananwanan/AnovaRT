// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(target_os = "none")]
core::arch::global_asm!(include_str!("startup.s"));

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
