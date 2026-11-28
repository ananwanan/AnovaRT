// SPDX-License-Identifier: Apache-2.0
use anova_cortex_m::{halt, interrupt};
use anova_kernel::memory::tlsf::Tlsf;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::{NonNull, null_mut},
    sync::atomic::{AtomicU32, Ordering::Relaxed},
};

struct GlobalHeap(UnsafeCell<Option<Tlsf<'static>>>);

// SAFETY: Only one core exists. All heap access is in Thread Mode with
// PRIMASK saved and restored. No closure calls back into the allocator.
unsafe impl Sync for GlobalHeap {}

#[global_allocator]
static HEAP: GlobalHeap = GlobalHeap(UnsafeCell::new(None));

/// Number of rejected Handler Mode allocations or deallocations.
#[unsafe(no_mangle)]
static M3_HANDLER_REJECTIONS: AtomicU32 = AtomicU32::new(0);

pub(super) fn with<R>(operation: impl FnOnce(&mut Tlsf<'static>) -> R) -> Option<R> {
    if interrupt::in_interrupt_context() {
        M3_HANDLER_REJECTIONS.fetch_add(1, Relaxed);
        return None;
    }
    interrupt::critical_section(|| {
        // SAFETY: Thread Mode exclusion and no reentry during the closure.
        unsafe { (&mut *HEAP.0.get()).as_mut().map(operation) }
    })
}

#[inline(never)]
pub(super) fn initialize() {
    interrupt::critical_section(|| {
        // SAFETY: M3 calls this once at boot, before alloc users or IRQ dispatch.
        let heap = Tlsf::new(unsafe { anova_board::memory::heap_backing() }).expect("TLSF arena");
        // SAFETY: No other code has access to the slot at this point.
        unsafe { HEAP.0.get().write(Some(heap)) };
    });
}

#[cfg(feature = "m3-test-tlsf")]
pub(super) fn request_in_handler() {
    // SAFETY: GlobalAlloc accepts this valid nonzero layout. Handler Mode
    // must reject it before touching the allocator.
    let address = unsafe { HEAP.alloc(Layout::new::<u32>()) };
    if !address.is_null() {
        halt();
    }
}

#[cfg(feature = "m3-test-tlsf")]
pub(super) fn handler_rejections() -> u32 {
    M3_HANDLER_REJECTIONS.load(Relaxed)
}

// SAFETY: The TLSF core returns disjoint payloads conforming to Layout and
// accepts only still-live allocations from this arena under allocator exclusion.
unsafe impl GlobalAlloc for GlobalHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        with(|heap| heap.allocate(layout).ok())
            .flatten()
            .map_or(null_mut(), NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if with(|heap| {
            // SAFETY: GlobalAlloc requires the original live pointer and Layout.
            unsafe { heap.deallocate(NonNull::new_unchecked(ptr)) }.is_ok()
        }) != Some(true)
        {
            halt();
        }
    }
}
