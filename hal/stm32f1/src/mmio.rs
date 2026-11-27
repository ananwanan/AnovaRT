// SPDX-License-Identifier: Apache-2.0

pub(crate) unsafe fn read(address: usize) -> u32 {
    // SAFETY: The caller guarantees an aligned, readable 32-bit MMIO register.
    unsafe { core::ptr::read_volatile(core::ptr::without_provenance(address)) }
}

pub(crate) unsafe fn write(address: usize, value: u32) {
    // SAFETY: The caller guarantees an aligned, writable 32-bit MMIO register.
    unsafe { core::ptr::write_volatile(core::ptr::without_provenance_mut(address), value) }
}
