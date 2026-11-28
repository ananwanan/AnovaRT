// SPDX-License-Identifier: Apache-2.0
use anova_kernel::memory::{
    MemoryError,
    pool::MemoryPool,
    stack::{self, StackRegion},
    tlsf::{HEADER, MAX_REGION, MIN_BLOCK, Tlsf},
};
use core::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

#[repr(align(1024))]
struct PoolBacking([MaybeUninit<u8>; 4096]);

#[test]
fn pool_all_classes_reuse_and_reject_invalid_addresses() {
    let mut region = PoolBacking([MaybeUninit::uninit(); 4096]);
    assert!(matches!(
        MemoryPool::new(&mut region.0[1..33], 32),
        Err(MemoryError::InvalidRegion)
    ));
    assert!(matches!(
        MemoryPool::new(&mut region.0[..31], 32),
        Err(MemoryError::InvalidRegion)
    ));
    assert!(matches!(
        MemoryPool::new(&mut region.0[..0], 32),
        Err(MemoryError::InvalidRegion)
    ));
    assert!(matches!(
        MemoryPool::new(&mut region.0, 48),
        Err(MemoryError::InvalidRegion)
    ));
    for size in [16, 32, 64, 128, 256, 512, 1024] {
        let base = region.0.as_mut_ptr().cast::<u8>();
        let mut pool = MemoryPool::new(&mut region.0, size).unwrap();
        let count = 4096 / size;
        assert_eq!(pool.available(), count);
        let mut allocations = Vec::new();
        for index in 0..count {
            let ptr = pool.allocate().unwrap();
            assert_eq!(ptr.addr().get() % size, 0);
            assert_eq!(pool.index(ptr), Some(index));
            assert_eq!(pool.address(index), Some(ptr));
            allocations.push(ptr);
        }
        assert_eq!(pool.address(count), None);
        assert_eq!(pool.allocate(), Err(MemoryError::Exhausted));
        assert_eq!(pool.stats().used, count);
        assert_eq!(pool.stats().peak_used, count);
        // SAFETY: These addresses fail the pool's boundary or range check.
        unsafe {
            assert_eq!(
                pool.deallocate(NonNull::new(base.wrapping_add(1)).unwrap()),
                Err(MemoryError::InvalidBlock)
            );
            assert_eq!(
                pool.deallocate(NonNull::new(base.wrapping_add(4096)).unwrap()),
                Err(MemoryError::InvalidBlock)
            );
        }
        for ptr in allocations.into_iter().rev() {
            // SAFETY: Each address is released once, then tested while free.
            unsafe {
                assert_eq!(pool.deallocate(ptr), Ok(()));
                assert_eq!(pool.deallocate(ptr), Err(MemoryError::DoubleFree));
            }
        }
        assert_eq!(pool.available(), count);
        assert_eq!(pool.stats().peak_used, count);
        assert_eq!(pool.allocate().unwrap(), pool.address(0).unwrap());
    }
}

fn layout(size: usize, alignment: usize) -> Layout {
    Layout::from_size_align(size, alignment).unwrap()
}

#[test]
fn heap_initialization_exact_fit_and_small_remainder() {
    let mut short = [MaybeUninit::uninit(); (MIN_BLOCK + HEADER) / 8 - 1];
    assert!(matches!(
        Tlsf::new(&mut short),
        Err(MemoryError::InvalidRegion)
    ));
    let mut oversized = vec![MaybeUninit::uninit(); MAX_REGION / 8 + 1];
    assert!(matches!(
        Tlsf::new(&mut oversized),
        Err(MemoryError::InvalidRegion)
    ));
    let mut maximum = vec![MaybeUninit::uninit(); MAX_REGION / 8];
    let mut heap = Tlsf::new(&mut maximum).unwrap();
    let initial = heap.validate().unwrap();
    assert_eq!(initial.largest_free_block, MAX_REGION - 2 * HEADER);
    for slack in [0, 8, 16] {
        let ptr = heap
            .allocate(layout(initial.free_payload - slack, 8))
            .unwrap();
        let expected_free = if slack < MIN_BLOCK { 0 } else { slack };
        assert_eq!(heap.stats().free(), expected_free);
        assert_eq!(heap.stats().internal_padding(), slack - expected_free);
        assert_eq!(heap.allocate(layout(9, 8)), Err(MemoryError::Exhausted));
        // SAFETY: ptr is live and the only owned allocation.
        unsafe { heap.deallocate(ptr) }.unwrap();
        assert_eq!(heap.validate().unwrap(), initial);
    }
    assert_eq!(heap.allocate(layout(0, 1)), Err(MemoryError::InvalidLayout));
    assert_eq!(
        heap.allocate(layout(isize::MAX as usize, 1)),
        Err(MemoryError::Exhausted)
    );
    assert_eq!(
        heap.allocate(layout(1, 1 << (usize::BITS - 2))),
        Err(MemoryError::Exhausted)
    );
    assert_eq!(heap.stats().allocations, 3);
    assert_eq!(heap.stats().deallocations, 3);
    assert_eq!(heap.stats().failures, 6);
    assert_eq!(heap.stats().peak_used, MAX_REGION - HEADER);
}

#[test]
fn heap_alignment_coalescing_and_recovery() {
    let mut words = [MaybeUninit::uninit(); 1536];
    let mut heap = Tlsf::new(&mut words).unwrap();
    let initial = heap.validate().unwrap();
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [0, 2, 1, 3]] {
        for align in (0..=8).map(|power| 1 << power) {
            let pointers = [(); 4].map(|_| heap.allocate(layout(13, align)).unwrap());
            assert!(pointers.iter().all(|ptr| ptr.addr().get() % align == 0));
            for i in order {
                // SAFETY: Each index in the permutation is different and live.
                unsafe { heap.deallocate(pointers[i]) }.unwrap();
                heap.validate().unwrap();
            }
            assert_eq!(heap.validate().unwrap(), initial);
        }
    }
}

#[test]
fn heap_rejects_interior_and_checks_external_fragmentation() {
    let mut words = [MaybeUninit::uninit(); 128];
    let mut heap = Tlsf::new(&mut words).unwrap();
    let a = heap.allocate(layout(64, 8)).unwrap();
    let b = heap.allocate(layout(64, 8)).unwrap();
    let invalid = NonNull::new(a.as_ptr().wrapping_add(8)).unwrap();
    // SAFETY: invalid is inside a but does not denote any payload start.
    assert_eq!(
        unsafe { heap.deallocate(invalid) },
        Err(MemoryError::InvalidBlock)
    );
    // SAFETY: a is live, exclusively owned, and no payload is borrowed.
    unsafe { heap.deallocate(a) }.unwrap();
    // SAFETY: a still denotes a known free boundary and has no live owner.
    assert_eq!(unsafe { heap.deallocate(a) }, Err(MemoryError::DoubleFree));
    let holes = heap.validate().unwrap();
    assert_eq!(holes.external_fragmentation(), (64, holes.free_payload));
    // SAFETY: b is still live, exclusively owned, and unborrowed.
    unsafe { heap.deallocate(b) }.unwrap();
    let recovered = heap.validate().unwrap();
    assert_eq!(recovered.external_fragmentation().0, 0);
    let fill = heap.allocate(layout(recovered.free_payload, 8)).unwrap();
    assert_eq!(heap.validate().unwrap().external_fragmentation(), (0, 1));
    // SAFETY: fill is the only live allocation and no payload is borrowed.
    unsafe { heap.deallocate(fill) }.unwrap();
}

#[derive(Clone, Copy)]
struct Live {
    ptr: NonNull<u8>,
    len: usize,
    tag: u8,
}

fn check_payload(block: Live) {
    // SAFETY: The stress table owns the live initialized payload.
    let bytes = unsafe { core::slice::from_raw_parts(block.ptr.as_ptr(), block.len) };
    assert!(bytes.iter().all(|&byte| byte == block.tag));
}

#[test]
fn heap_randomized_100000_operations_with_payload_integrity() {
    let mut words = [MaybeUninit::uninit(); 1536];
    let start = words.as_ptr() as usize;
    let end = start + core::mem::size_of_val(&words);
    let mut heap = Tlsf::new(&mut words).unwrap();
    let initial = heap.validate().unwrap();
    let mut slots: [Option<Live>; 36] = [None; 36];
    let mut seed = 0x173d_c1b5u32;
    for operation in 0..100_000 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let index = seed as usize % slots.len();
        if let Some(block) = slots[index].take() {
            check_payload(block);
            // SAFETY: Only this slot owned the allocation and it is now empty.
            unsafe { heap.deallocate(block.ptr) }.unwrap();
        } else {
            let size = match operation % 4 {
                0 => 13,
                1 => 997,
                _ => 1 + (seed >> 8) as usize % 1024,
            };
            let align = 1 << ((seed >> 25) % 9);
            match heap.allocate(layout(size, align)) {
                Ok(ptr) => {
                    let address = ptr.addr().get();
                    assert_eq!(address % align, 0);
                    assert!(start <= address && address + size <= end);
                    for other in slots.iter().flatten() {
                        let address2 = other.ptr.addr().get();
                        assert!(address + size <= address2 || address2 + other.len <= address);
                    }
                    let tag = seed as u8;
                    // SAFETY: This newly allocated payload contains size bytes.
                    unsafe { ptr.as_ptr().write_bytes(tag, size) };
                    slots[index] = Some(Live {
                        ptr,
                        len: size,
                        tag,
                    });
                }
                Err(MemoryError::Exhausted) => {
                    for &block in slots.iter().flatten() {
                        check_payload(block);
                    }
                }
                Err(error) => panic!("unexpected error {error:?}"),
            }
        }
        heap.validate().unwrap();
        if operation % 97 == 0 {
            for &block in slots.iter().flatten() {
                check_payload(block);
            }
        }
    }
    assert!(heap.stats().failures > 0);
    for block in slots.into_iter().flatten() {
        check_payload(block);
        // SAFETY: Each remaining allocation is owned by a distinct slot.
        unsafe { heap.deallocate(block.ptr) }.unwrap();
    }
    assert_eq!(heap.validate().unwrap(), initial);
    assert_eq!(heap.stats().used, 0);
    assert_eq!(heap.stats().allocated_blocks, 0);
}

#[repr(align(8))]
struct StackBacking([u32; 64]);

#[test]
fn stack_watermark_canary_and_address_bounds() {
    let mut storage = StackBacking([0; 64]);
    let words = &mut storage.0;
    let bottom = words.as_ptr() as usize;
    let top = bottom + 256;
    assert!(StackRegion::new(bottom + 4, top).is_err());
    assert!(StackRegion::new(top, bottom).is_err());
    assert!(stack::fill_stack(&mut []).is_err());
    let region = stack::fill_stack(words).unwrap();
    assert_eq!(region.inspect(words, top).unwrap().remaining, 248);
    assert_eq!(region.inspect(words, top).unwrap().peak_used, 0);
    assert!(region.inspect(&words[2..], top).is_err());
    words[24..].fill(1);
    assert_eq!(region.inspect(words, top).unwrap().remaining, 88);
    assert_eq!(region.inspect(words, bottom + 80).unwrap().peak_used, 176);
    words[2..].fill(1);
    assert_eq!(region.inspect(words, top).unwrap().remaining, 0);
    assert!(region.inspect(words, top).unwrap().canary_intact);
    words[1] = 0;
    assert!(!region.inspect(words, top).unwrap().canary_intact);
    assert!(region.contains_stack_pointer(bottom + 8));
    for invalid in [bottom, bottom + 4, top - 4, top + 8] {
        assert!(!region.inspect(words, invalid).unwrap().pointer_valid);
    }
}
