// SPDX-License-Identifier: Apache-2.0
//! Privileged single-core interrupt controls.

use crate::nvic::Priority;
use core::arch::asm;

macro_rules! read_special {
    ($register:literal) => {{
        let value: u32;
        // SAFETY: MRS reads a core register and changes no machine state.
        unsafe { asm!(concat!("mrs {}, ", $register), out(reg) value, options(nostack, preserves_flags)) };
        value
    }};
}

pub fn interrupts_enabled() -> bool {
    read_special!("PRIMASK") == 0
}
pub fn exception_number() -> u16 {
    read_special!("IPSR") as u16
}
pub fn in_interrupt_context() -> bool {
    exception_number() != 0
}
pub fn basepri() -> u8 {
    (read_special!("BASEPRI") >> 4) as u8
}

pub fn dmb() {
    // SAFETY: A full-system barrier does not change memory or flags.
    unsafe { asm!("dmb", options(nostack, preserves_flags)) };
}

pub fn dsb() {
    // SAFETY: Complete previous explicit accesses.
    unsafe { asm!("dsb", options(nostack, preserves_flags)) };
}

pub fn isb() {
    // SAFETY: Synchronize the instruction stream with prior control writes.
    unsafe { asm!("isb", options(nostack, preserves_flags)) };
}

pub fn synchronize() {
    dsb();
    isb();
}

pub fn mask() {
    // SAFETY: Firmware is privileged. The memory clobber prevents compiler
    // motion across the mask, and the barrier orders subsequent accesses.
    unsafe { asm!("cpsid i", options(nostack, preserves_flags)) };
    synchronize();
}

/// # Safety
/// VTOR, stacks and enabled handlers must be ready before dispatch can occur.
pub unsafe fn unmask() {
    // SAFETY: The caller guarantees initialized interrupt infrastructure.
    unsafe { asm!("dsb", "cpsie i", "isb", options(nostack, preserves_flags)) };
}

struct Restore<const BASE: bool>(u32);

impl<const BASE: bool> Drop for Restore<BASE> {
    fn drop(&mut self) {
        // SAFETY: Restore the exact saved value on the same privileged core.
        // The asm memory clobber and barriers bracket normal closure execution.
        unsafe {
            if BASE {
                asm!("dsb", "msr BASEPRI, {}", "isb", in(reg) self.0, options(nostack, preserves_flags));
            } else {
                asm!("dsb", "msr PRIMASK, {}", "isb", in(reg) self.0, options(nostack, preserves_flags));
            }
        }
    }
}

/// Mask configurable exceptions on this core. NMI and HardFault can still run.
/// A terminal panic does not unwind or restore the previous mask.
pub fn critical_section<R>(f: impl FnOnce() -> R) -> R {
    let saved = read_special!("PRIMASK");
    mask();
    let _restore = Restore::<false>(saved);
    f()
}

/// Mask priorities at or below the specified urgency. Zero adds no mask.
/// BASEPRI_MAX preserves a stronger outer threshold.
pub fn with_priority_ceiling<R>(priority: Priority, f: impl FnOnce() -> R) -> R {
    let saved = read_special!("BASEPRI");
    // SAFETY: The priority value is encoded for the implemented bits; the
    // write can only strengthen the mask. The guard restores it on return.
    unsafe {
        asm!("msr BASEPRI_MAX, {}", in(reg) u32::from(priority.encoded()), options(nostack, preserves_flags));
    }
    synchronize();
    let _restore = Restore::<true>(saved);
    f()
}
