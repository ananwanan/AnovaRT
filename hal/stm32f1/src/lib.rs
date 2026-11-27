// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(target_os = "none")]
mod mmio;

#[cfg(any(target_os = "none", test))]
pub mod gpio;

#[cfg(target_os = "none")]
core::arch::global_asm!(include_str!("vectors.s"));
