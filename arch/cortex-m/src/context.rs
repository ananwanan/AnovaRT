// SPDX-License-Identifier: Apache-2.0
//! Cortex-M3 basic hardware frame and software-saved r4 through r11.

use crate::fault::ExceptionFrame;

pub type Entry = extern "C" fn(u32);
pub type Exit = extern "C" fn() -> !;

pub const THREAD_PSP: u32 = 0xffff_fffd;
pub const THREAD_MSP: u32 = 0xffff_fff9;
pub const SOFTWARE_BYTES: usize = 8 * size_of::<u32>();
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
