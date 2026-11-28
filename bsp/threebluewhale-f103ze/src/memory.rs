// SPDX-License-Identifier: Apache-2.0
//! STM32F103ZET6 linker region addresses.

use core::{mem::MaybeUninit, ops::Range, slice};

unsafe extern "C" {
    static __sram_start: u8;
    static __sram_end: u8;
    static __data_start: u8;
    static __bss_end: u8;
    static __pool_start: u8;
    static __pool_end: u8;
    static __heap_start: u8;
    static __heap_end: u8;
    static __stack_bottom: u8;
    static __stack_top: u8;
    static __fault_stack_bottom: u8;
    static __fault_stack_top: u8;
}

pub struct Regions {
    pub sram: Range<usize>,
    pub static_ram: Range<usize>,
    pub pool: Range<usize>,
    pub heap: Range<usize>,
    pub main_stack: Range<usize>,
    pub fault_stack: Range<usize>,
}

pub fn regions() -> Regions {
    Regions {
        sram: &raw const __sram_start as usize..&raw const __sram_end as usize,
        static_ram: &raw const __data_start as usize..&raw const __bss_end as usize,
        pool: &raw const __pool_start as usize..&raw const __pool_end as usize,
        heap: &raw const __heap_start as usize..&raw const __heap_end as usize,
        main_stack: &raw const __stack_bottom as usize..&raw const __stack_top as usize,
        fault_stack: &raw const __fault_stack_bottom as usize
            ..&raw const __fault_stack_top as usize,
    }
}

/// # Safety
/// Call once with the linker-provided pool range and exclusive access to it.
pub unsafe fn pool_backing() -> &'static mut [MaybeUninit<u8>] {
    // SAFETY: Linker reserves this disjoint, writable range. The caller
    // guarantees unique ownership, and the bytes may be uninitialized.
    unsafe {
        slice::from_raw_parts_mut(
            (&raw const __pool_start).cast_mut().cast(),
            regions().pool.len(),
        )
    }
}

/// # Safety
/// Call once with the linker-provided heap range and exclusive access to it.
pub unsafe fn heap_backing() -> &'static mut [MaybeUninit<u64>] {
    // SAFETY: Linker reserves this disjoint, writable, 8-byte-aligned range.
    // The caller guarantees unique ownership, and the words may be uninitialized.
    unsafe {
        slice::from_raw_parts_mut(
            (&raw const __heap_start).cast_mut().cast(),
            regions().heap.len() / 8,
        )
    }
}

/// Executable addresses for saved exception return validation.
pub fn code_range() -> Range<usize> {
    unsafe extern "C" {
        static __text_start: u8;
        static __text_end: u8;
    }
    (&raw const __text_start).addr()..(&raw const __text_end).addr()
}
