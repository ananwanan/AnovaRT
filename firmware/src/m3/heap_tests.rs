// SPDX-License-Identifier: Apache-2.0
use alloc::{boxed::Box, string::String, vec::Vec};
use anova_kernel::memory::{MemoryError, tlsf::HeapDiagnostics};
use core::{alloc::Layout, ptr::NonNull};

use super::{ALIGNMENT, BoardLog, FRAGMENTATION, OOM, TLSF, global};

fn alloc(size: usize, align: usize) -> Result<NonNull<u8>, MemoryError> {
    global::with(|heap| heap.allocate(Layout::from_size_align(size, align).unwrap())).unwrap()
}

fn release(address: NonNull<u8>) {
    // SAFETY: Callers pass a live allocation they exclusively own and finished using.
    global::with(|heap| unsafe { heap.deallocate(address) })
        .unwrap()
        .unwrap();
}

fn diagnostics() -> HeapDiagnostics {
    global::with(|heap| heap.validate()).unwrap().unwrap()
}

pub(super) fn run(sink: &mut BoardLog<'_>) {
    let initial = diagnostics();
    #[cfg(feature = "m3-test-corruption")]
    {
        global::with(|heap| heap.verify_corruption_detection())
            .unwrap()
            .unwrap();
        report!(sink, "[M3 PASS] TLSF malformed link and boundary detected");
    }
    let value = Box::new(core::hint::black_box(0x1234_5678));
    assert_eq!(**core::hint::black_box(&value), 0x1234_5678);
    let mut vector = Vec::new();
    let mut text = String::new();
    for i in 0..128 {
        vector.push(i);
        text.push(char::from(b'a' + (i % 26) as u8));
    }
    assert!(vector.iter().copied().eq(0..128));
    assert!(text.bytes().eq((0..128).map(|i| b'a' + (i % 26) as u8)));
    drop((value, vector, text));
    report!(sink, "[M3 PASS] Box Vec String and growth");

    for power in 0..=if ALIGNMENT { 8 } else { 4 } {
        let align = 1 << power;
        let block = alloc(31, align).unwrap();
        assert_eq!(block.addr().get() % align, 0);
        release(block);
    }
    report!(sink, "[M3 PASS] TLSF alignment");

    let [a, b, c] = [(); 3].map(|_| alloc(40, 8).unwrap());
    assert_eq!(
        diagnostics().largest_free_block,
        initial.largest_free_block - 144
    );
    release(a);
    release(c);
    assert_eq!(diagnostics().free_payload, initial.free_payload - 56);
    release(b);
    assert_eq!(diagnostics(), initial);
    report!(sink, "[M3 PASS] TLSF split and bidirectional coalesce");

    if OOM {
        let full = alloc(initial.free_payload, 8).unwrap();
        assert_eq!(alloc(1, 8), Err(MemoryError::Exhausted));
        release(full);
        let recovered = alloc(1, 8).unwrap();
        release(recovered);
    } else {
        assert_eq!(
            alloc(initial.free_payload + 1, 8),
            Err(MemoryError::Exhausted)
        );
    }
    let recovered = diagnostics();
    assert_eq!(recovered, initial);
    let (fragmented, free) = recovered.external_fragmentation();
    report!(
        sink,
        "[M3] TLSF largest_free={} free_payload={} external_fragmentation={}/{}",
        recovered.largest_free_block,
        recovered.free_payload,
        fragmented,
        free
    );
    report!(sink, "[M3 PASS] TLSF exhaustion and recovery");

    if TLSF || FRAGMENTATION {
        let operations = 10_000;
        stress(operations, FRAGMENTATION);
        assert_eq!(diagnostics(), initial);
        report!(
            sink,
            "[M3 PASS] TLSF stress operations={} alternating={}",
            operations,
            FRAGMENTATION
        );
    }
}

#[derive(Clone, Copy)]
struct Allocation {
    address: NonNull<u8>,
    length: usize,
    tag: u8,
}

impl Allocation {
    fn verify(self) {
        // SAFETY: The stress table owns this live, initialized payload.
        let bytes = unsafe { core::slice::from_raw_parts(self.address.as_ptr(), self.length) };
        assert!(bytes.iter().all(|&byte| byte == self.tag));
    }
}

fn stress(operations: usize, alternating: bool) {
    let mut random = 0x173d_c1b5u32;
    let mut active: [Option<Allocation>; 24] = [None; 24];
    let heap = anova_board::memory::regions().heap;
    for i in 0..operations {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let slot = random as usize % active.len();
        if let Some(block) = active[slot].take() {
            block.verify();
            release(block.address);
        } else {
            let length = if alternating {
                if i % 2 == 0 { 13 } else { 997 }
            } else {
                1 + ((random >> 8) as usize % 700)
            };
            let align = 1 << ((random >> 28) % 9);
            if let Ok(address) = alloc(length, align) {
                let start = address.addr().get();
                assert_eq!(start % align, 0);
                assert!(heap.start <= start && start + length <= heap.end);
                for existing in active.iter().flatten() {
                    let other = existing.address.addr().get();
                    assert!(start + length <= other || other + existing.length <= start);
                }
                let tag = random as u8;
                // SAFETY: TLSF granted this exclusive payload for length bytes.
                unsafe { address.as_ptr().write_bytes(tag, length) };
                active[slot] = Some(Allocation {
                    address,
                    length,
                    tag,
                });
            } else {
                for block in active.iter().flatten() {
                    block.verify();
                }
            }
        }
        if i % 97 == 0 {
            diagnostics();
            for block in active.iter().flatten() {
                block.verify();
            }
        }
    }
    for block in active.into_iter().flatten() {
        block.verify();
        release(block.address);
    }
}
