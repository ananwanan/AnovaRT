// SPDX-License-Identifier: Apache-2.0
//! Terminal Cortex-M3 exception snapshots.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stack {
    Main,
    Process,
}

pub fn exception_stack(exc_return: u32) -> Option<Stack> {
    match exc_return {
        0xffff_fff1 | 0xffff_fff9 => Some(Stack::Main),
        0xffff_fffd => Some(Stack::Process),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExceptionFrame {
    pub r0: u32,
    pub r1: u32,
    pub r2: u32,
    pub r3: u32,
    pub r12: u32,
    pub lr: u32,
    pub pc: u32,
    pub xpsr: u32,
}

const FRAME_SIZE: u32 = size_of::<ExceptionFrame>() as u32;
const STACK_ERRORS: u32 = (3 << 3) | (3 << 11);
const STACK_PADDING: u32 = 1 << 9;

fn in_sram(address: u32, size: u32, start: u32, end: u32) -> bool {
    address >= start && address.checked_add(size).is_some_and(|limit| limit <= end)
}

pub fn frame_address(
    exc_return: u32,
    msp: u32,
    psp: u32,
    cfsr: u32,
    start: u32,
    end: u32,
) -> Option<u32> {
    let address = match exception_stack(exc_return)? {
        Stack::Main => msp,
        Stack::Process => psp,
    };
    (cfsr & STACK_ERRORS == 0 && address & 7 == 0 && in_sram(address, FRAME_SIZE, start, end))
        .then_some(address)
}

pub fn valid_padding(address: u32, xpsr: u32, start: u32, end: u32) -> bool {
    let size = FRAME_SIZE + if xpsr & STACK_PADDING != 0 { 4 } else { 0 };
    in_sram(address, size, start, end)
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Status {
    pub cfsr: u32,
    pub hfsr: u32,
    pub dfsr: u32,
    pub afsr: u32,
    pub mmfar: u32,
    pub bfar: u32,
    pub shcsr: u32,
    pub icsr: u32,
}

impl Status {
    pub fn memory_address(self) -> Option<u32> {
        (self.cfsr & (1 << 7) != 0).then_some(self.mmfar)
    }

    pub fn bus_address(self) -> Option<u32> {
        (self.cfsr & (1 << 15) != 0).then_some(self.bfar)
    }

    pub fn reasons(self) -> impl Iterator<Item = &'static str> {
        [
            (self.cfsr & 1, "Instruction access violation"),
            (self.cfsr & 2, "Data access violation"),
            (self.cfsr & (1 << 8), "Instruction bus fault"),
            (self.cfsr & (1 << 9), "Precise bus fault"),
            (self.cfsr & (1 << 10), "Imprecise bus fault"),
            (self.cfsr & ((1 << 3) | (1 << 11)), "Unstacking fault"),
            (self.cfsr & ((1 << 4) | (1 << 12)), "Stacking fault"),
            (self.cfsr & (1 << 16), "Undefined instruction"),
            (self.cfsr & (1 << 17), "Invalid state"),
            (self.cfsr & (1 << 18), "Invalid PC"),
            (self.cfsr & (1 << 19), "Coprocessor unavailable"),
            (self.cfsr & (1 << 24), "Unaligned access"),
            (self.cfsr & (1 << 25), "Divide by zero"),
            (self.hfsr & (1 << 30), "Forced HardFault"),
            (self.hfsr & (1 << 1), "Vector table fault"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| (bit != 0).then_some(name))
    }
}

#[cfg(target_os = "none")]
mod hardware {
    use super::*;
    use crate::{interrupt, registers as reg};

    pub fn configure() {
        const STKALIGN: u32 = 1 << 9;
        const DIV_0_TRP: u32 = 1 << 4;
        const UNALIGN_TRP: u32 = 1 << 3;
        let ccr = 0xe000_ed14;
        let required = reg::read(ccr) | STKALIGN | DIV_0_TRP;
        // Rust/LLVM may combine byte accesses into unaligned word accesses
        // on thumbv7m. M3 uses the architectural behavior for normal SRAM.
        let configured = if cfg!(feature = "m3") {
            required & !UNALIGN_TRP
        } else {
            required | UNALIGN_TRP
        };
        reg::write(ccr, configured);
        // Startup owns SHCSR; publish all three configurable fault enables.
        reg::write(0xe000_ed24, 7 << 16);
        interrupt::synchronize();
    }

    pub fn disable_usage_fault() {
        interrupt::critical_section(|| {
            let shcsr = 0xe000_ed24;
            reg::write(shcsr, reg::read(shcsr) & !(1 << 18));
            interrupt::synchronize();
        });
    }

    impl Status {
        pub fn read() -> Self {
            Self {
                cfsr: reg::read(0xe000_ed28),
                hfsr: reg::read(0xe000_ed2c),
                dfsr: reg::read(0xe000_ed30),
                afsr: reg::read(0xe000_ed3c),
                mmfar: reg::read(0xe000_ed34),
                bfar: reg::read(0xe000_ed38),
                shcsr: reg::read(0xe000_ed24),
                icsr: reg::read(0xe000_ed04),
            }
        }
    }
}

#[cfg(target_os = "none")]
pub use hardware::{configure, disable_usage_fault};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_abi_and_exc_return() {
        assert_eq!(size_of::<ExceptionFrame>(), 32);
        assert_eq!(align_of::<ExceptionFrame>(), 4);
        assert_eq!(core::mem::offset_of!(ExceptionFrame, lr), 20);
        assert_eq!(core::mem::offset_of!(ExceptionFrame, pc), 24);
        assert_eq!(core::mem::offset_of!(ExceptionFrame, xpsr), 28);
        assert_eq!(exception_stack(0xffff_fff1), Some(Stack::Main));
        assert_eq!(exception_stack(0xffff_fff9), Some(Stack::Main));
        assert_eq!(exception_stack(0xffff_fffd), Some(Stack::Process));
        assert_eq!(exception_stack(0xffff_ffe9), None);
        assert_eq!(exception_stack(0xffff_fff5), None);
    }

    #[test]
    fn rejects_corrupt_or_truncated_frames() {
        let start = 0x2000_0000;
        let end = start + 0x10000;
        let last = end - 32;
        let frame = |msp, cfsr| frame_address(0xffff_fff9, msp, 0, cfsr, start, end);
        assert_eq!(frame(last, 0), Some(last));
        assert_eq!(frame(start - 8, 0), None);
        assert_eq!(frame(start + 4, 0), None);
        assert_eq!(frame(last + 8, 0), None);
        for bit in [3, 4, 11, 12] {
            assert_eq!(frame(last, 1 << bit), None);
        }
        assert_eq!(
            frame_address(0xffff_fffd, 0, last, 0, start, end),
            Some(last)
        );
        assert_eq!(
            frame_address(0xffff_fff9, u32::MAX - 7, 0, 0, 0, u32::MAX),
            None
        );
        assert!(valid_padding(last, 0, start, end));
        assert!(!valid_padding(last, STACK_PADDING, start, end));
        assert!(valid_padding(last - 8, STACK_PADDING, start, end));
    }

    #[test]
    fn fault_flags_and_address_validity() {
        let mut s = Status {
            cfsr: (1 << 16) | (1 << 9),
            hfsr: 1 << 30,
            dfsr: 0,
            afsr: 0,
            mmfar: 1,
            bfar: 2,
            shcsr: 0,
            icsr: 0,
        };
        let mut reasons = s.reasons();
        assert_eq!(reasons.next(), Some("Precise bus fault"));
        assert_eq!(reasons.next(), Some("Undefined instruction"));
        assert_eq!(reasons.next(), Some("Forced HardFault"));
        assert_eq!(reasons.next(), None);
        assert_eq!(s.memory_address(), None);
        assert_eq!(s.bus_address(), None);
        s.cfsr |= (1 << 7) | (1 << 15);
        assert_eq!(s.memory_address(), Some(1));
        assert_eq!(s.bus_address(), Some(2));
    }
}
