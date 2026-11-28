// SPDX-License-Identifier: Apache-2.0
//! Portable memory management. Shared instances require external synchronization.

pub mod pool;
pub mod stack;
pub mod tlsf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryError {
    InvalidRegion,
    InvalidLayout,
    InvalidBlock,
    DoubleFree,
    Exhausted,
    Corrupt,
}
