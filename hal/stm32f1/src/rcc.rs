// SPDX-License-Identifier: Apache-2.0
//! STM32F103 clock setup from the reset state (RM0008).
const HSI_HZ: u32 = 8_000_000;
const HSION: u32 = 1;
const HSIRDY: u32 = 1 << 1;
const HSEON: u32 = 1 << 16;
const HSERDY: u32 = 1 << 17;
const PLLON: u32 = 1 << 24;
const PLLRDY: u32 = 1 << 25;
const SW: u32 = 3;
const SWS: u32 = 3 << 2;
const PLL: u32 = 2;
const BUS_MASK: u32 = (15 << 4) | (7 << 8) | (7 << 11);
const APB1_DIV2: u32 = 4 << 8;
const PLL_MASK: u32 = (1 << 16) | (1 << 17) | (15 << 18);
const HSE_TIMES_9: u32 = (1 << 16) | (7 << 18);
const LATENCY: u32 = 7;
const PREFETCH: u32 = 1 << 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Hsi,
    HsePll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clocks {
    pub source: Source,
    pub sysclk_hz: u32,
    pub hclk_hz: u32,
    pub pclk1_hz: u32,
    pub pclk2_hz: u32,
}

impl Clocks {
    fn from_config(source: Source, sysclk_hz: u32, cfgr: u32) -> Self {
        let ahb_shift = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 6, 7, 8, 9];
        let apb_shift = [0, 0, 0, 0, 1, 2, 3, 4];
        let hclk_hz = sysclk_hz >> ahb_shift[((cfgr >> 4) & 15) as usize];
        Self {
            source,
            sysclk_hz,
            hclk_hz,
            pclk1_hz: hclk_hz >> apb_shift[((cfgr >> 8) & 7) as usize],
            pclk2_hz: hclk_hz >> apb_shift[((cfgr >> 11) & 7) as usize],
        }
    }
}

/// The supported 72 MHz profile requires a confirmed 8 MHz crystal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalClock {
    Unconfirmed,
    Crystal8Mhz,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    None,
    HseTimeout,
    PllTimeout,
    SwitchTimeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    UnconfirmedCrystal,
    HseTimeout,
    PllTimeout,
    SwitchTimeout,
    FlashConfiguration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    HsiNotReady,
    HsiSwitchFailed,
    PllStopFailed,
    HseStopFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockInit {
    pub clocks: Clocks,
    pub fallback: Option<Fallback>,
}

#[derive(Clone, Copy)]
enum Register {
    Cr,
    Cfgr,
    Flash,
}

trait Registers {
    fn read(&mut self, register: Register) -> u32;
    fn write(&mut self, register: Register, value: u32);

    fn update(&mut self, register: Register, mask: u32, value: u32) {
        let previous = self.read(register);
        self.write(register, (previous & !mask) | value);
    }
}

fn wait(
    io: &mut impl Registers,
    register: Register,
    mask: u32,
    expected: u32,
    budget: u32,
) -> bool {
    (0..budget).any(|_| io.read(register) & mask == expected)
}

fn restore_hsi(io: &mut impl Registers, budget: u32) -> Result<Clocks, ClockError> {
    io.update(Register::Cr, HSION, HSION);
    if !wait(io, Register::Cr, HSIRDY, HSIRDY, budget) {
        return Err(ClockError::HsiNotReady);
    }
    io.update(Register::Cfgr, SW, 0);
    if !wait(io, Register::Cfgr, SWS, 0, budget) {
        return Err(ClockError::HsiSwitchFailed);
    }
    io.update(Register::Cr, PLLON, 0);
    if !wait(io, Register::Cr, PLLRDY, 0, budget) {
        return Err(ClockError::PllStopFailed);
    }
    io.update(Register::Cr, HSEON, 0);
    if !wait(io, Register::Cr, HSERDY, 0, budget) {
        return Err(ClockError::HseStopFailed);
    }
    io.update(Register::Cfgr, BUS_MASK, 0);
    io.update(Register::Flash, LATENCY, 0);
    Ok(Clocks::from_config(
        Source::Hsi,
        HSI_HZ,
        io.read(Register::Cfgr),
    ))
}

fn try_pll(io: &mut impl Registers, fault: Fault, budget: u32) -> Result<Clocks, Fallback> {
    io.update(Register::Cr, HSEON, HSEON);
    let ready = wait(io, Register::Cr, HSERDY, HSERDY, budget);
    if !ready || fault == Fault::HseTimeout {
        return Err(Fallback::HseTimeout);
    }

    // Set prefetch at HCLK /1 and raise latency before increasing HCLK.
    io.update(Register::Flash, LATENCY | PREFETCH, 2 | PREFETCH);
    if io.read(Register::Flash) & (LATENCY | PREFETCH) != (2 | PREFETCH) {
        return Err(Fallback::FlashConfiguration);
    }
    io.update(Register::Cfgr, BUS_MASK | PLL_MASK, APB1_DIV2 | HSE_TIMES_9);
    io.update(Register::Cr, PLLON, PLLON);
    let ready = wait(io, Register::Cr, PLLRDY, PLLRDY, budget);
    if !ready || fault == Fault::PllTimeout {
        return Err(Fallback::PllTimeout);
    }

    io.update(Register::Cfgr, SW, PLL);
    let switched = wait(io, Register::Cfgr, SWS, PLL << 2, budget);
    if !switched || fault == Fault::SwitchTimeout {
        return Err(Fallback::SwitchTimeout);
    }
    Ok(Clocks::from_config(
        Source::HsePll,
        8_000_000 * 9,
        io.read(Register::Cfgr),
    ))
}

fn configure(
    io: &mut impl Registers,
    external: ExternalClock,
    fault: Fault,
    budget: u32,
) -> Result<ClockInit, ClockError> {
    let hsi = restore_hsi(io, budget)?;
    if external == ExternalClock::Unconfirmed {
        return Ok(ClockInit {
            clocks: hsi,
            fallback: Some(Fallback::UnconfirmedCrystal),
        });
    }
    match try_pll(io, fault, budget) {
        Ok(clocks) => Ok(ClockInit {
            clocks,
            fallback: None,
        }),
        Err(reason) => Ok(ClockInit {
            clocks: restore_hsi(io, budget)?,
            fallback: Some(reason),
        }),
    }
}

#[cfg(target_os = "none")]
mod hardware {
    use super::*;
    use crate::mmio;

    const RCC: usize = 0x4002_1000;

    /// The clock gates used by the active board.
    #[derive(Clone, Copy)]
    pub enum Peripheral {
        GpioA,
        GpioB,
        GpioE,
        Usart1,
    }

    /// Exclusive owner of clock configuration during startup.
    pub struct Rcc(());

    impl Rcc {
        /// # Safety
        /// Call once on STM32F103 from hardware reset, with interrupts masked.
        /// No other code may access RCC or Flash ACR concurrently.
        pub unsafe fn after_reset() -> Self {
            Self(())
        }

        pub fn configure(
            &mut self,
            external: ExternalClock,
            fault: Fault,
        ) -> Result<ClockInit, ClockError> {
            configure(self, external, fault, 1_000_000)
        }

        pub fn enable(&mut self, peripheral: Peripheral) {
            apb2_gate(peripheral, true);
        }

        /// # Safety
        /// The peripheral must be quiescent and have no live driver handles.
        pub unsafe fn disable(&mut self, peripheral: Peripheral) {
            apb2_gate(peripheral, false);
        }
    }

    fn apb2_gate(peripheral: Peripheral, enabled: bool) {
        let mask = 1
            << match peripheral {
                Peripheral::GpioA => 2,
                Peripheral::GpioB => 3,
                Peripheral::GpioE => 6,
                Peripheral::Usart1 => 14,
            };
        let address = RCC + 0x18;
        // SAFETY: Rcc owns the aligned APB2 gate register; readback completes the write.
        unsafe {
            let old = mmio::read(address);
            mmio::write(address, (old & !mask) | if enabled { mask } else { 0 });
            let _ = mmio::read(address);
        }
    }

    impl Register {
        fn address(self) -> usize {
            match self {
                Self::Cr => RCC,
                Self::Cfgr => RCC + 4,
                Self::Flash => 0x4002_2000,
            }
        }
    }

    impl Registers for Rcc {
        fn read(&mut self, register: Register) -> u32 {
            // SAFETY: Rcc owns these aligned 32-bit registers.
            unsafe { mmio::read(register.address()) }
        }

        fn write(&mut self, register: Register, value: u32) {
            // SAFETY: The startup sequence modifies only defined writable fields.
            unsafe { mmio::write(register.address(), value) }
        }
    }
}

#[cfg(target_os = "none")]
pub use hardware::{Peripheral, Rcc};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Model {
        cr: u32,
        cfgr: u32,
        flash: u32,
        fail: Option<Fault>,
        refuse_hsi: bool,
    }

    impl Registers for Model {
        fn read(&mut self, register: Register) -> u32 {
            match register {
                Register::Cr => self.cr,
                Register::Cfgr => self.cfgr,
                Register::Flash => self.flash,
            }
        }

        fn write(&mut self, register: Register, value: u32) {
            match register {
                Register::Cr => {
                    self.cr = value & !(HSIRDY | HSERDY | PLLRDY);
                    if value & HSION != 0 {
                        self.cr |= HSIRDY;
                    }
                    if value & HSEON != 0 && self.fail != Some(Fault::HseTimeout) {
                        self.cr |= HSERDY;
                    }
                    if value & PLLON != 0 && self.fail != Some(Fault::PllTimeout) {
                        self.cr |= PLLRDY;
                    }
                }
                Register::Cfgr => {
                    let requested = value & SW;
                    let status = if (requested == PLL && self.fail == Some(Fault::SwitchTimeout))
                        || (requested == 0 && self.refuse_hsi)
                    {
                        self.cfgr & SWS
                    } else {
                        requested << 2
                    };
                    if status == PLL << 2 {
                        assert_eq!(self.flash & (LATENCY | PREFETCH), 2 | PREFETCH);
                        assert_eq!(value & BUS_MASK, APB1_DIV2);
                        assert_eq!(self.cr & PLLRDY, PLLRDY);
                    }
                    self.cfgr = (value & !SWS) | status;
                }
                Register::Flash => {
                    if value & LATENCY == 0 {
                        assert_eq!(self.cfgr & SWS, 0);
                    }
                    self.flash = value;
                }
            }
        }
    }

    #[test]
    fn pll_clocks_and_unrelated_fields() {
        let mut io = Model {
            cfgr: 1 << 22,
            cr: 1 << 7,
            ..Model::default()
        };
        let init = configure(&mut io, ExternalClock::Crystal8Mhz, Fault::None, 3).unwrap();
        assert_eq!(
            init.clocks,
            Clocks {
                source: Source::HsePll,
                sysclk_hz: 72_000_000,
                hclk_hz: 72_000_000,
                pclk1_hz: 36_000_000,
                pclk2_hz: 72_000_000
            }
        );
        assert_eq!(init.fallback, None);
        assert_ne!(io.cfgr & (1 << 22), 0);
        assert_ne!(io.cr & (1 << 7), 0);
        let divided = Clocks::from_config(Source::HsePll, 72_000_000, (8 << 4) | (5 << 11));
        assert_eq!(
            (divided.hclk_hz, divided.pclk1_hz, divided.pclk2_hz),
            (36_000_000, 36_000_000, 9_000_000)
        );
    }

    #[test]
    fn failures_and_injections_restore_hsi() {
        for (fault, reason) in [
            (Fault::HseTimeout, Fallback::HseTimeout),
            (Fault::PllTimeout, Fallback::PllTimeout),
            (Fault::SwitchTimeout, Fallback::SwitchTimeout),
        ] {
            for synthetic in [false, true] {
                let mut io = Model {
                    fail: (!synthetic).then_some(fault),
                    ..Model::default()
                };
                let init = configure(
                    &mut io,
                    ExternalClock::Crystal8Mhz,
                    if synthetic { fault } else { Fault::None },
                    3,
                )
                .unwrap();
                assert_eq!(init.fallback, Some(reason));
                assert_eq!(init.clocks.source, Source::Hsi);
                assert_eq!(
                    [
                        init.clocks.sysclk_hz,
                        init.clocks.hclk_hz,
                        init.clocks.pclk1_hz,
                        init.clocks.pclk2_hz
                    ],
                    [HSI_HZ; 4]
                );
                assert_eq!(io.cr & (HSEON | PLLON), 0);
                assert_eq!(io.cfgr & (SW | SWS | BUS_MASK), 0);
                assert_eq!(io.flash & LATENCY, 0);
            }
        }
    }

    #[test]
    fn unconfirmed_crystal_and_failed_recovery() {
        let mut io = Model::default();
        let init = configure(&mut io, ExternalClock::Unconfirmed, Fault::None, 3).unwrap();
        assert_eq!(init.fallback, Some(Fallback::UnconfirmedCrystal));
        assert_eq!(io.cr & (HSEON | PLLON), 0);
        try_pll(&mut io, Fault::None, 3).unwrap();
        io.refuse_hsi = true;
        assert_eq!(restore_hsi(&mut io, 3), Err(ClockError::HsiSwitchFailed));
    }
}
