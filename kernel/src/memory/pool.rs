// SPDX-License-Identifier: Apache-2.0
use core::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use super::MemoryError;

const END: u16 = u16::MAX;
const BITMAP_WORDS: usize = 32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PoolStats {
    pub block_size: usize,
    pub capacity: usize,
    pub used: usize,
    pub peak_used: usize,
}

/// Fixed blocks aligned to their size. The free-list link occupies the first
/// two bytes of each free block. The bitmap belongs to the pool controller.
pub struct MemoryPool<'a> {
    base: NonNull<u8>,
    head: u16,
    allocated: [u32; BITMAP_WORDS],
    stats: PoolStats,
    backing: PhantomData<&'a mut [MaybeUninit<u8>]>,
}

impl<'a> MemoryPool<'a> {
    pub fn new(region: &'a mut [MaybeUninit<u8>], block_size: usize) -> Result<Self, MemoryError> {
        if !(16..=1024).contains(&block_size)
            || !block_size.is_power_of_two()
            || !region.len().is_multiple_of(block_size)
            || !(region.as_ptr() as usize).is_multiple_of(block_size)
        {
            return Err(MemoryError::InvalidRegion);
        }
        let capacity = region.len() / block_size;
        if !(1..=BITMAP_WORDS * 32).contains(&capacity) {
            return Err(MemoryError::InvalidRegion);
        }
        let pool = Self {
            base: NonNull::from(region).cast(),
            head: 0,
            allocated: [0; BITMAP_WORDS],
            stats: PoolStats {
                block_size,
                capacity,
                used: 0,
                peak_used: 0,
            },
            backing: PhantomData,
        };
        for index in 0..capacity {
            pool.write_link(
                index,
                if index + 1 == capacity {
                    END
                } else {
                    (index + 1) as u16
                },
            );
        }
        Ok(pool)
    }

    pub fn address(&self, index: usize) -> Option<NonNull<u8>> {
        if index >= self.stats.capacity {
            return None;
        }
        // SAFETY: A checked index selects a block inside the exclusive region.
        Some(unsafe { self.base.add(index * self.stats.block_size) })
    }

    pub fn index(&self, address: NonNull<u8>) -> Option<usize> {
        let offset = address.addr().get().checked_sub(self.base.addr().get())?;
        let index = offset / self.stats.block_size;
        (index < self.stats.capacity && offset.is_multiple_of(self.stats.block_size))
            .then_some(index)
    }

    fn write_link(&self, index: usize, next: u16) {
        // SAFETY: The checked block is free and aligned for its two-byte link.
        unsafe { self.address(index).unwrap().cast::<u16>().write(next) };
    }

    pub fn allocate(&mut self) -> Result<NonNull<u8>, MemoryError> {
        if self.head == END {
            return Err(MemoryError::Exhausted);
        }
        let index = usize::from(self.head);
        let address = self.address(index).unwrap();
        // SAFETY: The free-list head is a free block with an initialized link.
        self.head = unsafe { address.cast::<u16>().read() };
        self.allocated[index / 32] |= 1 << (index % 32);
        self.stats.used += 1;
        self.stats.peak_used = self.stats.peak_used.max(self.stats.used);
        Ok(address)
    }

    /// # Safety
    /// If the address denotes a live block, the caller must own that allocation
    /// and end its use before this call. Addresses alone cannot identify reuse
    /// of a slot by a later allocation.
    pub unsafe fn deallocate(&mut self, address: NonNull<u8>) -> Result<(), MemoryError> {
        let index = self.index(address).ok_or(MemoryError::InvalidBlock)?;
        let mask = 1 << (index % 32);
        if self.allocated[index / 32] & mask == 0 {
            return Err(MemoryError::DoubleFree);
        }
        self.allocated[index / 32] &= !mask;
        self.write_link(index, self.head);
        self.head = index as u16;
        self.stats.used -= 1;
        Ok(())
    }

    pub fn stats(&self) -> PoolStats {
        self.stats
    }

    pub fn available(&self) -> usize {
        self.stats.capacity - self.stats.used
    }
}
