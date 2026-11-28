// SPDX-License-Identifier: Apache-2.0
//! Exclusive static task stack loans and architecture-independent diagnostics.

use crate::memory::stack::{CANARY, GUARD_BYTES, PATTERN, StackRegion, StackStats, fill_stack};
use core::ptr::NonNull;

pub const MIN_BYTES: usize = 512;
pub const DEFAULT_BYTES: usize = 1024;

#[repr(C, align(8))]
pub struct StackStorage<const WORDS: usize = { DEFAULT_BYTES / 4 }>(pub [u32; WORDS]);

/// Owns the loan until the task has been reaped. It never exposes a reference
/// over a running or suspended task's live frames.
pub struct Stack {
    words: NonNull<u32>,
    bottom: usize,
    top: usize,
    region: StackRegion,
}

impl Stack {
    pub fn new(words: &'static mut [u32]) -> Result<Self, &'static mut [u32]> {
        let bytes = core::mem::size_of_val(words);
        let bottom = words.as_mut_ptr().addr();
        let Some(top) = bottom.checked_add(bytes) else {
            return Err(words);
        };
        if bytes < MIN_BYTES || StackRegion::new(bottom, top).is_err() {
            return Err(words);
        }
        let region = fill_stack(words).unwrap();
        let words = NonNull::new(words.as_mut_ptr()).unwrap();
        Ok(Self {
            words,
            bottom,
            top,
            region,
        })
    }

    pub fn top(&self) -> usize {
        self.top
    }

    /// Whether an aligned frame beginning at sp fits above the guard.
    pub fn fits(&self, sp: usize, bytes: usize) -> bool {
        self.region.contains_stack_pointer(sp)
            && sp.checked_add(bytes).is_some_and(|end| end <= self.top)
    }

    /// Check the guard without reading live frame words.
    pub fn canary_intact(&self) -> bool {
        // SAFETY: The exclusive stack loan includes both initialized guard
        // words. Valid task stack accesses stay above this reservation.
        unsafe {
            self.words.as_ptr().read_volatile() == CANARY
                && self.words.as_ptr().add(1).read_volatile() == CANARY
        }
    }

    /// # Safety
    /// The task must be suspended (or never started), and sp must be its
    /// saved pointer. The task cannot resume until the scan is finished.
    pub unsafe fn usage(&self, sp: usize) -> StackStats {
        let intact = self.canary_intact();
        let mut cursor = self.bottom + GUARD_BYTES;
        if self.fits(sp, 0) && intact {
            while cursor < sp {
                // SAFETY: This word lies within the initialized pattern area
                // below the saved context and no task can access it now.
                if unsafe { self.words.as_ptr().with_addr(cursor).read_volatile() } != PATTERN {
                    break;
                }
                cursor += 4;
            }
        }
        self.region.summarize(sp, cursor, intact)
    }

    /// # Safety
    /// The task has never run or has terminated and its context cannot resume.
    pub unsafe fn into_storage(self) -> &'static mut [u32] {
        // SAFETY: Ownership came from the exclusive static slice and no
        // execution frame can still borrow it by the caller's guarantee.
        unsafe {
            core::slice::from_raw_parts_mut(self.words.as_ptr(), (self.top - self.bottom) / 4)
        }
    }

    /// Returns a raw pointer with the original stack allocation's provenance.
    pub fn pointer(&self, address: usize) -> *mut u32 {
        self.words.as_ptr().with_addr(address)
    }
}
