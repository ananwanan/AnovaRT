// SPDX-License-Identifier: Apache-2.0
//! STM32F103 high-density external interrupt numbering.

pub type Irq = anova_cortex_m::nvic::Irq<60>;

pub const DMA1_CHANNEL1: Irq = match Irq::new(11) {
    Some(irq) => irq,
    None => unreachable!(),
};
pub const DMA1_CHANNEL2: Irq = match Irq::new(12) {
    Some(irq) => irq,
    None => unreachable!(),
};
pub const DMA1_CHANNEL3: Irq = match Irq::new(13) {
    Some(irq) => irq,
    None => unreachable!(),
};
