// SPDX-License-Identifier: Apache-2.0
use core::arch::asm;

pub struct Sample {
    pub sp: usize,
    pub first_used: usize,
    pub canary_intact: bool,
}

/// Sample the active descending MSP using a stackless scan.
///
/// # Safety
/// Call in privileged Thread Mode on MSP with configurable interrupts masked.
/// The supplied bounds must identify the live, reset-initialized MSP region.
/// NMI and HardFault must be terminal.
pub unsafe fn sample(bottom: usize, top: usize, pattern: u32, canary: u32) -> Sample {
    let sp: usize;
    let cursor: usize;
    let intact: u32;
    // SAFETY: The caller supplies the initialized bounds and excludes returning
    // exceptions. The assembly does not move SP or write the stack. LLVM is
    // informed that the assembly reads memory.
    unsafe {
        asm!(
            "mrs {sp}, MSP",
            "adds {cursor}, {bottom}, #8",
            "movs {intact}, #0",
            "cmp {sp}, {cursor}",
            "blo 3f",
            "cmp {sp}, {top}",
            "bhi 3f",
            "tst {sp}, #7",
            "bne 3f",
            "ldr {word}, [{bottom}]",
            "cmp {word}, {canary}",
            "bne 3f",
            "ldr {word}, [{bottom}, #4]",
            "cmp {word}, {canary}",
            "bne 3f",
            "movs {intact}, #1",
            "2:",
            "cmp {cursor}, {sp}",
            "bhs 3f",
            "ldr {word}, [{cursor}]",
            "cmp {word}, {pattern}",
            "bne 3f",
            "adds {cursor}, #4",
            "b 2b",
            "3:",
            sp = out(reg) sp,
            cursor = out(reg) cursor,
            intact = out(reg) intact,
            word = out(reg) _,
            bottom = in(reg) bottom,
            top = in(reg) top,
            canary = in(reg) canary,
            pattern = in(reg) pattern,
            options(nostack, readonly)
        );
    }
    Sample {
        sp,
        first_used: cursor,
        canary_intact: intact != 0,
    }
}
