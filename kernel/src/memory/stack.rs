// SPDX-License-Identifier: Apache-2.0
use super::MemoryError;

pub const PATTERN: u32 = 0xa5a5_a5a5;
pub const CANARY: u32 = 0xdead_beef;
pub const GUARD_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackRegion {
    bottom: usize,
    top: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackStats {
    pub size: usize,
    pub stack_pointer: usize,
    pub pointer_valid: bool,
    pub canary_intact: bool,
    pub remaining: usize,
    pub peak_used: usize,
}

pub fn fill_stack(words: &mut [u32]) -> Result<StackRegion, MemoryError> {
    let bottom = words.as_ptr() as usize;
    let region = StackRegion::new(bottom, bottom + core::mem::size_of_val(words))?;
    words[..2].fill(CANARY);
    words[2..].fill(PATTERN);
    Ok(region)
}

impl StackRegion {
    pub fn new(bottom: usize, top: usize) -> Result<Self, MemoryError> {
        if !bottom.is_multiple_of(8)
            || !top.is_multiple_of(8)
            || top.saturating_sub(bottom) <= GUARD_BYTES
        {
            return Err(MemoryError::InvalidRegion);
        }
        Ok(Self { bottom, top })
    }

    pub fn contains_stack_pointer(self, sp: usize) -> bool {
        sp.is_multiple_of(8) && (self.bottom + GUARD_BYTES..=self.top).contains(&sp)
    }

    pub fn summarize(self, sp: usize, first_used: usize, canary_intact: bool) -> StackStats {
        let pointer_valid = self.contains_stack_pointer(sp);
        let usable_bottom = self.bottom + GUARD_BYTES;
        let cursor = if pointer_valid && canary_intact {
            first_used.clamp(usable_bottom, sp)
        } else {
            usable_bottom
        };
        StackStats {
            size: self.top - self.bottom,
            stack_pointer: sp,
            pointer_valid,
            canary_intact,
            remaining: cursor - usable_bottom,
            peak_used: self.top - cursor,
        }
    }

    pub fn inspect(self, words: &[u32], sp: usize) -> Result<StackStats, MemoryError> {
        let bottom = words.as_ptr() as usize;
        if self.bottom != bottom || self.top - self.bottom != core::mem::size_of_val(words) {
            return Err(MemoryError::InvalidRegion);
        }
        let intact = words[..2] == [CANARY; 2];
        let first_used = if self.contains_stack_pointer(sp) && intact {
            let unused = words[2..(sp - bottom) / 4]
                .iter()
                .take_while(|&&word| word == PATTERN)
                .count();
            bottom + GUARD_BYTES + unused * 4
        } else {
            bottom + GUARD_BYTES
        };
        Ok(self.summarize(sp, first_used, intact))
    }
}
