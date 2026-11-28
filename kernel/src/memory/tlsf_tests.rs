// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn mapping_boundaries_and_search_rounding() {
    assert_eq!(mapping_insert(MIN_BLOCK), (0, 2));
    assert_eq!(
        mapping_insert(MAX_REGION - ALIGNMENT),
        (FL_COUNT - 1, SL_COUNT - 1)
    );
    assert_eq!(mapping_insert(56), (0, 7));
    assert_eq!(mapping_insert(64), (1, 0));
    assert_eq!(mapping_insert(120), (1, 7));
    assert_eq!(mapping_insert(128), (2, 0));
    assert_eq!(mapping_insert(136), (2, 0));
    assert_eq!(mapping_search(136), Some((2, 1)));
    assert_eq!(mapping_search(144), Some((2, 1)));
    assert_eq!(mapping_search(248), Some((3, 0)));
    assert_eq!(mapping_search(usize::MAX), None);
    assert_eq!(mapping_search(MAX_REGION), None);
    assert_eq!(mapping_search(MIN_BLOCK - 1), None);
    assert_eq!(align_up(usize::MAX, ALIGNMENT), None);
    for size in (MIN_BLOCK..MAX_REGION).step_by(ALIGNMENT) {
        if let Some(class) = mapping_search(size) {
            assert!(class.0 < FL_COUNT && class.1 < SL_COUNT);
            assert!(class >= mapping_insert(size));
            assert!(size == MIN_BLOCK || mapping_insert(size - ALIGNMENT) < class);
        }
    }
}

#[test]
fn lookup_and_corruption_detection() {
    let mut words = [MaybeUninit::uninit(); MAX_REGION / 8];
    let mut heap = Tlsf::new(&mut words).unwrap();
    let largest = usize::from(heap.end);
    assert_eq!(heap.locate(largest), Some(0));
    assert_eq!(heap.locate(MIN_BLOCK), Some(0));
    assert_eq!(heap.verify_corruption_detection(), Ok(()));
    let ptr = heap
        .allocate(Layout::from_size_align(largest - HEADER, 8).unwrap())
        .unwrap();
    assert_eq!(heap.fl, 0);
    assert_eq!(heap.locate(MIN_BLOCK), None);
    // SAFETY: ptr is the one live allocation and no payload is borrowed.
    unsafe { heap.deallocate(ptr) }.unwrap();
    heap.fl |= 1 << FL_COUNT;
    assert_eq!(heap.validate(), Err(MemoryError::Corrupt));
}
