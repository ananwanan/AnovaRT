// SPDX-License-Identifier: Apache-2.0
//! Cortex-M3 periodic timer, sourced from HCLK.

#[derive(Debug, PartialEq, Eq)]
pub enum ReloadError {
    ZeroRate,
    NonIntegral,
    OutOfRange,
}

pub fn reload(hclk_hz: u32, tick_hz: u32) -> Result<u32, ReloadError> {
    if hclk_hz == 0 || tick_hz == 0 {
        return Err(ReloadError::ZeroRate);
    }
    if !hclk_hz.is_multiple_of(tick_hz) {
        return Err(ReloadError::NonIntegral);
    }
    let cycles = hclk_hz / tick_hz;
    if !(2..=0x0100_0000).contains(&cycles) {
        return Err(ReloadError::OutOfRange);
    }
    Ok(cycles - 1)
}

#[cfg(target_os = "none")]
mod hardware {
    use super::*;
    use crate::{interrupt, registers as reg};

    const CTRL: usize = 0xe000_e010;
    const LOAD: usize = 0xe000_e014;
    const VAL: usize = 0xe000_e018;

    pub fn start(hclk_hz: u32, tick_hz: u32) -> Result<u32, ReloadError> {
        let value = reload(hclk_hz, tick_hz)?;
        interrupt::critical_section(|| {
            stop();
            reg::write(LOAD, value);
            reg::write(VAL, 0);
            reg::write(CTRL, 7);
            interrupt::synchronize();
        });
        Ok(value)
    }

    pub fn stop() {
        interrupt::critical_section(|| {
            reg::write(CTRL, 0);
            reg::write(0xe000_ed04, 1 << 25);
            interrupt::synchronize();
        });
    }

    /// Reading CTRL consumes COUNTFLAG. Tick accounting uses exception delivery.
    pub fn is_running() -> bool {
        reg::read(CTRL) & 1 != 0
    }
}

#[cfg(target_os = "none")]
pub use hardware::{is_running, start, stop};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_limits_and_clocks() {
        assert_eq!(reload(72_000_000, 1000), Ok(71_999));
        assert_eq!(reload(8_000_000, 1000), Ok(7_999));
        assert_eq!(reload(0, 1000), Err(ReloadError::ZeroRate));
        assert_eq!(reload(72_000_000, 0), Err(ReloadError::ZeroRate));
        assert_eq!(reload(72_000_001, 1000), Err(ReloadError::NonIntegral));
        assert_eq!(reload(1000, 1000), Err(ReloadError::OutOfRange));
        assert_eq!(reload(0x0100_0000, 1), Ok(0x00ff_ffff));
        assert_eq!(reload(0x0100_0001, 1), Err(ReloadError::OutOfRange));
    }
}
