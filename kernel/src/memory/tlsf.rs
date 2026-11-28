// SPDX-License-Identifier: Apache-2.0
//! Two-level segregated fit with 16-bit offsets within an exclusive arena.

use core::{alloc::Layout, marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use super::MemoryError;

pub const ALIGNMENT: usize = 8;
pub const HEADER: usize = 8;
pub const MIN_BLOCK: usize = 16;
pub const MAX_REGION: usize = 65536;
pub const FL_COUNT: usize = 11;
pub const SL_LOG2: usize = 3;
pub const SL_COUNT: usize = 1 << SL_LOG2;
const SMALL: usize = ALIGNMENT * SL_COUNT;
const NONE: u16 = u16::MAX;
const FREE: u16 = 1;
const SIZE: usize = 0;
const PREVIOUS: usize = 2;
const REQUESTED: usize = 4;
const PREV_LINK: usize = HEADER;
const NEXT_LINK: usize = HEADER + 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HeapStats {
    pub total: usize,
    pub used: usize,
    pub requested: usize,
    pub peak_used: usize,
    pub allocated_blocks: usize,
    pub free_blocks: usize,
    pub allocations: u32,
    pub deallocations: u32,
    pub failures: u32,
}

impl HeapStats {
    pub fn free(self) -> usize {
        self.total - HEADER - self.used
    }

    pub fn metadata(self) -> usize {
        HEADER * (1 + self.allocated_blocks + self.free_blocks)
            + 4 * self.free_blocks
            + core::mem::size_of::<Tlsf<'_>>()
    }

    pub fn internal_padding(self) -> usize {
        self.used - HEADER * self.allocated_blocks - self.requested
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeapDiagnostics {
    pub largest_free_block: usize,
    pub free_payload: usize,
}

impl HeapDiagnostics {
    /// Exact numerator and denominator for external fragmentation.
    /// With no free payload, the ratio is defined as zero.
    pub fn external_fragmentation(self) -> (usize, usize) {
        if self.free_payload == 0 {
            return (0, 1);
        }
        (
            self.free_payload - self.largest_free_block,
            self.free_payload,
        )
    }
}

pub struct Tlsf<'a> {
    base: NonNull<u8>,
    end: u16,
    fl: u16,
    sl: [u8; FL_COUNT],
    heads: [[u16; SL_COUNT]; FL_COUNT],
    boundaries: [u32; MAX_REGION / ALIGNMENT / 32],
    stats: HeapStats,
    backing: PhantomData<&'a mut [MaybeUninit<u64>]>,
}

fn mapping_insert(size: usize) -> (usize, usize) {
    if size < SMALL {
        return (0, size / ALIGNMENT);
    }
    let high_bit = (usize::BITS - 1 - size.leading_zeros()) as usize;
    let subdivision = size >> (high_bit - SL_LOG2);
    (
        high_bit - (SMALL.ilog2() as usize - 1),
        subdivision - SL_COUNT,
    )
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    Some(value.checked_add(alignment - 1)? & !(alignment - 1))
}

fn mapping_search(size: usize) -> Option<(usize, usize)> {
    if !(MIN_BLOCK..MAX_REGION).contains(&size) {
        return None;
    }
    let step = if size < SMALL {
        ALIGNMENT
    } else {
        1 << (size.ilog2() as usize - SL_LOG2)
    };
    let rounded = align_up(size, step)?;
    (rounded < MAX_REGION).then(|| mapping_insert(rounded))
}

impl<'a> Tlsf<'a> {
    pub fn new(region: &'a mut [MaybeUninit<u64>]) -> Result<Self, MemoryError> {
        let len = core::mem::size_of_val(region);
        if !(MIN_BLOCK + HEADER..=MAX_REGION).contains(&len)
            || !(region.as_ptr() as usize).is_multiple_of(ALIGNMENT)
        {
            return Err(MemoryError::InvalidRegion);
        }
        let end = (len - HEADER) as u16;
        let mut heap = Self {
            base: NonNull::from(region).cast(),
            end,
            fl: 0,
            sl: [0; FL_COUNT],
            heads: [[NONE; SL_COUNT]; FL_COUNT],
            boundaries: [0; MAX_REGION / ALIGNMENT / 32],
            stats: HeapStats {
                total: len,
                ..HeapStats::default()
            },
            backing: PhantomData,
        };
        heap.header(0, end, NONE, true, 0);
        heap.header(end, 0, 0, false, 0);
        heap.insert(0);
        Ok(heap)
    }

    fn word(&self, offset: u16, field: usize) -> u16 {
        // SAFETY: Internal offsets denote initialized words of live headers or
        // free links. Diagnostic callers validate offset and field first.
        unsafe {
            self.base
                .as_ptr()
                .add(usize::from(offset) + field)
                .cast::<u16>()
                .read()
        }
    }

    fn set(&mut self, offset: u16, field: usize, value: u16) {
        // SAFETY: The arena is exclusive and each field is aligned and inside
        // a header or the minimum payload of a free block.
        unsafe {
            self.base
                .as_ptr()
                .add(usize::from(offset) + field)
                .cast::<u16>()
                .write(value)
        };
    }

    fn boundary(&self, at: u16) -> bool {
        let index = usize::from(at) / ALIGNMENT;
        usize::from(at).is_multiple_of(ALIGNMENT)
            && self.boundaries[index / 32] & (1 << (index % 32)) != 0
    }

    fn clear_boundary(&mut self, at: u16) {
        let index = usize::from(at) / ALIGNMENT;
        self.boundaries[index / 32] &= !(1 << (index % 32));
    }

    fn header(&mut self, at: u16, size: u16, prev: u16, free: bool, requested: u16) {
        let index = usize::from(at) / ALIGNMENT;
        self.boundaries[index / 32] |= 1 << (index % 32);
        self.set(at, SIZE, size | u16::from(free));
        self.set(at, PREVIOUS, prev);
        self.set(at, REQUESTED, requested);
        self.set(at, 6, 0);
    }

    fn size(&self, at: u16) -> u16 {
        self.word(at, SIZE) & !7
    }

    fn free(&self, at: u16) -> bool {
        self.word(at, SIZE) & FREE != 0
    }

    fn insert(&mut self, at: u16) {
        let (fl, sl) = mapping_insert(usize::from(self.size(at)));
        let next = self.heads[fl][sl];
        self.set(at, PREV_LINK, NONE);
        self.set(at, NEXT_LINK, next);
        if next != NONE {
            self.set(next, PREV_LINK, at);
        }
        self.heads[fl][sl] = at;
        self.sl[fl] |= 1 << sl;
        self.fl |= 1 << fl;
        self.stats.free_blocks += 1;
    }

    fn remove(&mut self, at: u16) {
        let (fl, sl) = mapping_insert(usize::from(self.size(at)));
        let prev = self.word(at, PREV_LINK);
        let next = self.word(at, NEXT_LINK);
        if prev == NONE {
            self.heads[fl][sl] = next;
            if next == NONE {
                self.sl[fl] &= !(1 << sl);
                if self.sl[fl] == 0 {
                    self.fl &= !(1 << fl);
                }
            }
        } else {
            self.set(prev, NEXT_LINK, next);
        }
        if next != NONE {
            self.set(next, PREV_LINK, prev);
        }
        self.stats.free_blocks -= 1;
    }

    fn locate(&self, size: usize) -> Option<u16> {
        if size > usize::from(self.end) {
            return None;
        }
        // An exact or sufficiently large head in a partially covered class
        // can be used without rounding or searching that class's list.
        let (fl, sl) = mapping_insert(size);
        let head = self.heads[fl][sl];
        if head != NONE && usize::from(self.size(head)) >= size {
            return Some(head);
        }
        let (mut fl, sl) = mapping_search(size)?;
        let mut candidates = u16::from(self.sl[fl]) & (u16::MAX << sl);
        if candidates == 0 {
            let larger = self.fl & (u16::MAX << (fl + 1));
            if larger == 0 {
                return None;
            }
            fl = larger.trailing_zeros() as usize;
            candidates = u16::from(self.sl[fl]);
        }
        Some(self.heads[fl][candidates.trailing_zeros() as usize])
    }

    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, MemoryError> {
        let result = self.allocate_block(layout);
        if result.is_err() {
            self.stats.failures = self.stats.failures.saturating_add(1);
        }
        result
    }

    fn allocate_block(&mut self, layout: Layout) -> Result<NonNull<u8>, MemoryError> {
        if layout.size() == 0 {
            return Err(MemoryError::InvalidLayout);
        }
        let size = layout
            .size()
            .checked_add(HEADER)
            .and_then(|size| align_up(size, ALIGNMENT))
            .ok_or(MemoryError::InvalidLayout)?
            .max(MIN_BLOCK);
        let align = layout.align();
        let search = if align > ALIGNMENT {
            size.checked_add(align)
                .and_then(|size| size.checked_add(MIN_BLOCK - ALIGNMENT))
                .ok_or(MemoryError::InvalidLayout)?
        } else {
            size
        };
        let block = self.locate(search).ok_or(MemoryError::Exhausted)?;
        let block_end = block + self.size(block);
        let previous = self.word(block, PREVIOUS);
        let address = self.base.addr().get() + usize::from(block) + HEADER;
        let aligned = align_up(address, align).ok_or(MemoryError::InvalidLayout)?;
        let mut prefix = aligned - address;
        if prefix != 0 && prefix < MIN_BLOCK {
            prefix += align;
        }
        let at = block + prefix as u16;
        let tail = block_end - at - size as u16;
        self.remove(block);
        let previous = if prefix == 0 {
            previous
        } else {
            self.header(block, prefix as u16, previous, true, 0);
            self.insert(block);
            block
        };
        let used = if usize::from(tail) >= MIN_BLOCK {
            let remainder = at + size as u16;
            self.header(remainder, tail, at, true, 0);
            self.set(block_end, PREVIOUS, remainder);
            self.insert(remainder);
            size as u16
        } else {
            self.set(block_end, PREVIOUS, at);
            block_end - at
        };
        self.header(at, used, previous, false, layout.size() as u16);
        self.stats.used += usize::from(used);
        self.stats.requested += layout.size();
        self.stats.allocated_blocks += 1;
        self.stats.allocations = self.stats.allocations.saturating_add(1);
        self.stats.peak_used = self.stats.peak_used.max(self.stats.used);
        // SAFETY: The allocation contains this payload start, aligned as requested.
        Ok(unsafe { self.base.add(usize::from(at) + HEADER) })
    }

    /// # Safety
    /// If the address denotes a live payload, the caller must own that
    /// allocation and end all access to it. Range and boundary checks cannot
    /// distinguish a stale address from a later allocation at the same
    /// location. GlobalAlloc additionally requires the original Layout.
    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>) -> Result<(), MemoryError> {
        let offset = ptr
            .addr()
            .get()
            .checked_sub(self.base.addr().get())
            .and_then(|offset| offset.checked_sub(HEADER))
            .filter(|offset| *offset < usize::from(self.end) && offset.is_multiple_of(ALIGNMENT))
            .ok_or(MemoryError::InvalidBlock)?;
        let mut at = offset as u16;
        if !self.boundary(at) {
            return Err(MemoryError::InvalidBlock);
        }
        if self.free(at) {
            return Err(MemoryError::DoubleFree);
        }
        let mut size = self.size(at);
        self.stats.used -= usize::from(size);
        self.stats.requested -= usize::from(self.word(at, REQUESTED));
        self.stats.allocated_blocks -= 1;
        self.stats.deallocations = self.stats.deallocations.saturating_add(1);
        let mut prev = self.word(at, PREVIOUS);
        if prev != NONE && self.free(prev) {
            self.remove(prev);
            self.clear_boundary(at);
            size += self.size(prev);
            at = prev;
            prev = self.word(prev, PREVIOUS);
        }
        let next = at + size;
        if self.free(next) {
            self.remove(next);
            self.clear_boundary(next);
            size += self.size(next);
        }
        self.header(at, size, prev, true, 0);
        self.set(at + size, PREVIOUS, at);
        self.insert(at);
        Ok(())
    }

    pub fn stats(&self) -> HeapStats {
        self.stats
    }

    /// Inject and restore malformed metadata in a fully recovered test heap.
    #[cfg(any(test, feature = "memory-validation"))]
    pub fn verify_corruption_detection(&mut self) -> Result<(), MemoryError> {
        self.validate()?;
        if self.stats.allocated_blocks != 0 {
            return Err(MemoryError::InvalidRegion);
        }
        self.set(0, NEXT_LINK, 8);
        let invalid_link = self.validate();
        self.set(0, NEXT_LINK, NONE);
        let size = self.word(0, SIZE);
        self.set(0, SIZE, 16 | FREE);
        let invalid_boundary = self.validate();
        self.set(0, SIZE, size);
        if invalid_link != Err(MemoryError::Corrupt)
            || invalid_boundary != Err(MemoryError::Corrupt)
        {
            return Err(MemoryError::Corrupt);
        }
        self.validate().map(|_| ())
    }

    /// Check the physical chain, list ordering and class/bitmap consistency.
    /// Linear in the number of physical and listed blocks, plus table size.
    /// Requires stable metadata for the duration of the scan.
    pub fn validate(&self) -> Result<HeapDiagnostics, MemoryError> {
        let bad = MemoryError::Corrupt;
        let mut at = 0;
        let mut previous = NONE;
        let mut previous_free = false;
        let mut free_blocks = 0;
        let mut allocations = 0;
        let mut used = 0;
        let mut requested = 0;
        let mut result = HeapDiagnostics {
            largest_free_block: 0,
            free_payload: 0,
        };
        while at < self.end {
            if !self.boundary(at) || self.word(at, SIZE) & 6 != 0 {
                return Err(bad);
            }
            let size = self.size(at);
            let next = at
                .checked_add(size)
                .filter(|next| *next <= self.end)
                .ok_or(bad)?;
            if usize::from(size) < MIN_BLOCK || self.word(at, PREVIOUS) != previous {
                return Err(bad);
            }
            let free = self.free(at);
            let payload = usize::from(size) - HEADER;
            let request = usize::from(self.word(at, REQUESTED));
            if free {
                if previous_free || request != 0 {
                    return Err(bad);
                }
                result.largest_free_block = result.largest_free_block.max(payload);
                result.free_payload += payload;
                free_blocks += 1;
            } else {
                if request == 0 || request > payload {
                    return Err(bad);
                }
                requested += request;
                used += usize::from(size);
                allocations += 1;
            }
            previous_free = free;
            previous = at;
            at = next;
        }
        if self.word(self.end, SIZE) != 0
            || self.word(self.end, PREVIOUS) != previous
            || (free_blocks, allocations, used, requested)
                != (
                    self.stats.free_blocks,
                    self.stats.allocated_blocks,
                    self.stats.used,
                    self.stats.requested,
                )
        {
            return Err(bad);
        }
        let boundaries: usize = self
            .boundaries
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum();
        if boundaries != free_blocks + allocations + 1 {
            return Err(bad);
        }
        let mut linked = 0;
        let mut first_levels = 0;
        for (fl, heads) in self.heads.iter().enumerate() {
            let mut second_levels = 0;
            for (sl, &head) in heads.iter().enumerate() {
                let mut node = head;
                let mut prev = NONE;
                if node != NONE {
                    second_levels |= 1 << sl;
                }
                while node != NONE {
                    if node >= self.end
                        || !self.boundary(node)
                        || !self.free(node)
                        || mapping_insert(usize::from(self.size(node))) != (fl, sl)
                        || self.word(node, PREV_LINK) != prev
                        || linked == free_blocks
                    {
                        return Err(bad);
                    }
                    linked += 1;
                    prev = node;
                    node = self.word(node, NEXT_LINK);
                }
            }
            if second_levels != self.sl[fl] {
                return Err(bad);
            }
            if second_levels != 0 {
                first_levels |= 1 << fl;
            }
        }
        if linked != free_blocks || self.fl != first_levels {
            return Err(bad);
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "tlsf_tests.rs"]
mod tests;
