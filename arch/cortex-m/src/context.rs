// SPDX-License-Identifier: Apache-2.0
//! Cortex-M3 basic hardware frame and software-saved r4 through r11.

use crate::fault::ExceptionFrame;

pub type Entry = extern "C" fn(u32);
pub type Exit = extern "C" fn() -> !;

pub const THREAD_PSP: u32 = 0xffff_fffd;
pub const THREAD_MSP: u32 = 0xffff_fff9;
pub const SOFTWARE_BYTES: usize = size_of::<[u32; 8]>();
pub const THUMB: u32 = 1 << 24;
pub const ALIGN_PADDING: u32 = 1 << 9;

#[repr(C)]
pub struct InitialFrame {
    pub callee: [u32; 8],
    pub exception: ExceptionFrame,
}

pub const FRAME_BYTES: usize = size_of::<InitialFrame>();
pub const PC_OFFSET: usize =
    core::mem::offset_of!(InitialFrame, exception) + core::mem::offset_of!(ExceptionFrame, pc);
pub const XPSR_OFFSET: usize =
    core::mem::offset_of!(InitialFrame, exception) + core::mem::offset_of!(ExceptionFrame, xpsr);

const _: () = {
    assert!(FRAME_BYTES == 64);
    assert!(core::mem::offset_of!(InitialFrame, exception) == SOFTWARE_BYTES);
    assert!(PC_OFFSET == 56);
    assert!(XPSR_OFFSET == 60);
};

impl InitialFrame {
    /// The function addresses carry the Thumb bit. Stacked PC is even and
    /// xPSR.T establishes Thumb execution. LR is a normal branch target.
    pub fn new(entry: u32, exit: u32, argument: u32) -> Option<Self> {
        if entry & exit & 1 == 0 {
            return None;
        }
        Some(Self {
            callee: [0; 8],
            exception: ExceptionFrame {
                r0: argument,
                lr: exit,
                pc: entry & !1,
                xpsr: THUMB,
                ..ExceptionFrame::default()
            },
        })
    }
}

pub fn valid_thread_frame(pc: u32, xpsr: u32) -> bool {
    pc & 1 == 0 && xpsr & (THUMB | 0x1ff) == THUMB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_exception_image() {
        let frame = InitialFrame::new(0x0800_0101, 0x0800_0201, 42).unwrap();
        assert_eq!(frame.callee, [0; 8]);
        assert_eq!(frame.exception.r0, 42);
        assert_eq!(frame.exception.pc, 0x0800_0100);
        assert_eq!(frame.exception.lr, 0x0800_0201);
        assert_eq!(frame.exception.xpsr, THUMB);
        assert!(valid_thread_frame(frame.exception.pc, frame.exception.xpsr));
        assert!(!valid_thread_frame(frame.exception.pc, THUMB | 14));
        assert!(!valid_thread_frame(frame.exception.pc | 1, THUMB));
        assert!(InitialFrame::new(0x100, 0x201, 0).is_none());
        assert!(InitialFrame::new(0x101, 0x200, 0).is_none());
    }
}

#[cfg(all(target_os = "none", feature = "m4"))]
core::arch::global_asm!(
    include_str!("context_switch.s"),
    thread_msp = const THREAD_MSP,
    thread_psp = const THREAD_PSP,
);

/// Enter the installed M4 SVC bootstrap. It abandons the caller's MSP frames.
///
/// # Safety
/// Call once from privileged, unmasked Thread/MSP with BASEPRI and FAULTMASK
/// clear and with all initialized boot data moved into static storage.
#[cfg(all(target_os = "none", feature = "m4"))]
pub unsafe fn start() -> ! {
    // SAFETY: The caller has armed the one-shot SVC bootstrap and owns the
    // startup call chain. The handler enters a different, initialized frame.
    unsafe { core::arch::asm!("svc #0", "udf #4", options(noreturn)) }
}

#[cfg(all(target_os = "none", feature = "m4"))]
pub fn process_stack_pointer() -> usize {
    let value: usize;
    // SAFETY: Reading PSP changes no machine state.
    unsafe { core::arch::asm!("mrs {}, psp", out(reg) value, options(nostack, preserves_flags)) };
    value
}

#[cfg(all(target_os = "none", feature = "m4"))]
pub fn uses_process_stack() -> bool {
    let value: u32;
    // SAFETY: Reading CONTROL changes no machine state.
    unsafe {
        core::arch::asm!("mrs {}, CONTROL", out(reg) value, options(nostack, preserves_flags))
    };
    value & 2 != 0
}

#[cfg(all(target_os = "none", feature = "m4"))]
pub fn faultmask_clear() -> bool {
    let value: u32;
    // SAFETY: Reading FAULTMASK changes no machine state.
    unsafe {
        core::arch::asm!("mrs {}, FAULTMASK", out(reg) value, options(nostack, preserves_flags))
    };
    value == 0
}

#[cfg(all(target_os = "none", feature = "m4"))]
pub mod timing {
    unsafe extern "C" {
        static mut m4_cycle_trace: [u32; 6];
    }

    /// Enable sampling while the positively probed DWT session is live.
    pub fn enable(_counter: &crate::cycles::CycleCounter) {
        crate::interrupt::critical_section(|| {
            // SAFETY: The handler is excluded and this is the aligned flag.
            unsafe { (&raw mut m4_cycle_trace).cast::<u32>().write_volatile(1) };
        });
    }

    /// Read save, restore and PendSV-software cycle deltas after a switch.
    pub fn samples() -> [u32; 3] {
        crate::interrupt::critical_section(|| {
            let base = (&raw const m4_cycle_trace).cast::<u32>();
            // SAFETY: The handler is excluded and all indices are in the
            // .bss trace block. Reads are volatile across assembly writes.
            let [entry, save, saved, restore, restored] =
                unsafe { [1, 2, 3, 4, 5].map(|index| base.add(index).read_volatile()) };
            [
                saved.wrapping_sub(save),
                restored.wrapping_sub(restore),
                restored.wrapping_sub(entry),
            ]
        })
    }

    /// Read CYCCNT after the caller has verified a live DWT session.
    pub fn time() -> u32 {
        crate::registers::read(0xe000_1004)
    }
}

#[cfg(all(target_os = "none", feature = "m4"))]
pub fn main_stack_pointer() -> usize {
    let value: usize;
    // SAFETY: Reading MSP changes no machine state.
    unsafe { core::arch::asm!("mrs {}, msp", out(reg) value, options(nostack, preserves_flags)) };
    value
}

/// Construct a new task image in its exclusive stack storage.
///
/// # Safety
/// The aligned destination must have room for the entire Frame, be writable,
/// and belong to a task that cannot execute yet.
#[cfg(all(target_os = "none", feature = "m4"))]
pub unsafe fn write_initial(dst: *mut u32, entry: Entry, exit: Exit, argument: u32) {
    let frame = InitialFrame::new(entry as usize as u32, exit as usize as u32, argument)
        .expect("Thumb function pointers");
    // SAFETY: The caller guarantees an aligned, exclusive destination.
    unsafe { dst.cast::<InitialFrame>().write(frame) };
}

/// Return the required software plus hardware frame size, including padding.
///
/// # Safety
/// The PC and xPSR words at the architecture offsets must be initialized,
/// aligned and readable for the duration of this call.
#[cfg(all(target_os = "none", feature = "m4"))]
pub unsafe fn frame_bytes(frame: *const u32, text: core::ops::Range<usize>) -> Option<usize> {
    // SAFETY: Only the documented, caller-checked words are read. Volatile
    // accesses observe hardware exception stacking and assembly writes.
    let (pc, xpsr) = unsafe {
        (
            frame.add(PC_OFFSET / 4).read_volatile(),
            frame.add(XPSR_OFFSET / 4).read_volatile(),
        )
    };
    let padding = usize::from(xpsr & ALIGN_PADDING != 0) * 4;
    (text.contains(&(pc as usize)) && valid_thread_frame(pc, xpsr)).then_some(FRAME_BYTES + padding)
}
