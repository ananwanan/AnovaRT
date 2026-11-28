// SPDX-License-Identifier: Apache-2.0
use anova_board::{self as board, rcc::Fault, usart::Usart1};
use anova_kernel::{info, log::TextSink, warn};
use core::{
    fmt::{self, Write},
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

static CONSOLE_READY: AtomicBool = AtomicBool::new(false);
static PANICKING: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "m2"))]
pub fn run(mut led: board::Led) -> ! {
    let hse_fault = cfg!(feature = "m1-fault-hse");
    let pll_fault = cfg!(feature = "m1-fault-pll");
    let switch_fault = cfg!(feature = "m1-fault-switch");
    let panic_test = cfg!(feature = "m1-panic");
    // Accept Cargo feature unification for host tooling, but use a fixed
    // precedence on target. The documented test builds select one mode.
    let fault = if hse_fault {
        Fault::HseTimeout
    } else if pll_fault {
        Fault::PllTimeout
    } else if switch_fault {
        Fault::SwitchTimeout
    } else {
        Fault::None
    };

    let (init, mut serial) = initialize(fault, true);
    let clocks = init.clocks;
    if panic_test {
        panic!("M1 panic diagnostic test");
    }

    // Keep polling RX between small LED delay slices. This is a manual
    // single-byte echo test, with no queue or streaming guarantee.
    let slices_per_toggle = clocks.hclk_hz / 8_000;
    let mut remaining = 0;
    let mut on = false;
    loop {
        if remaining == 0 {
            on = !on;
            led.set(on);
            remaining = slices_per_toggle;
        }
        echo(&mut serial);
        anova_cortex_m::delay_iterations(1_000);
        remaining -= 1;
    }
}

pub(crate) fn initialize(
    fault: Fault,
    use_console: bool,
) -> (board::rcc::ClockInit, Option<Usart1>) {
    // SAFETY: Startup calls this once, with configurable interrupts masked.
    // Initial LED setup completed all RCC accesses before ownership transfers.
    let mut rcc = unsafe { board::rcc::Rcc::after_reset() };
    let Ok(init) = board::init_clock(&mut rcc, fault) else {
        anova_cortex_m::halt();
    };
    // SAFETY: This is the only initialization of the selected pins and USART1.
    // Clock and pin configuration remain fixed once the console is live.
    let mut serial = if use_console {
        unsafe { board::init_console(&mut rcc, init.clocks) }
            .ok()
            .flatten()
    } else {
        None
    };
    if let Some(uart) = serial.as_mut() {
        CONSOLE_READY.store(true, Ordering::Release);
        let _ = banner(uart, init);
    }
    (init, serial)
}

pub(crate) fn echo(serial: &mut Option<Usart1>) {
    if let Some(uart) = serial.as_mut() {
        match uart.receive() {
            Ok(Some(byte)) => {
                let _ = uart.send(byte);
            }
            Ok(None) => {}
            Err(error) => {
                let _ = warn!(&mut TextSink(uart), "USART1 RX {:?}", error);
            }
        }
    }
}

fn banner(uart: &mut Usart1, init: board::rcc::ClockInit) -> fmt::Result {
    let mut log = TextSink(uart);
    info!(&mut log, "AnovaRT boot on {}", board::NAME)?;
    info!(
        &mut log,
        "CPU {} | Flash {} KiB | SRAM {} KiB",
        board::MCU,
        board::FLASH_KIB,
        board::SRAM_KIB
    )?;
    if let Some(reason) = init.fallback {
        warn!(&mut log, "HSI fallback: {:?}", reason)?;
    }
    let clocks = init.clocks;
    info!(
        &mut log,
        "{:?} SYSCLK={} HCLK={} PCLK1={} PCLK2={} Hz",
        clocks.source,
        clocks.sysclk_hz,
        clocks.hclk_hz,
        clocks.pclk1_hz,
        clocks.pclk2_hz
    )?;
    info!(
        &mut log,
        "Startup memory OK | DS0 ready | USART1 {} 8N1 | single-byte echo ready",
        board::CONSOLE_BAUD
    )?;
    uart.flush().map_err(|_| fmt::Error)
}

pub(crate) struct TerminalWriter {
    uart: Usart1,
    bytes_left: usize,
}

impl Write for TerminalWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if self.bytes_left == 0 {
                return Err(fmt::Error);
            }
            self.bytes_left -= 1;
            self.uart.send(byte).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

pub(crate) fn terminal_output() -> Option<TerminalWriter> {
    anova_cortex_m::interrupt::mask();
    if PANICKING.swap(true, Ordering::AcqRel) || !CONSOLE_READY.load(Ordering::Acquire) {
        return None;
    }
    // SAFETY: Readiness publishes completed USART initialization. This
    // terminal path never resumes firmware, and ISRs do not use USART1.
    let uart = unsafe { Usart1::for_terminal_diagnostics() };
    Some(TerminalWriter {
        uart,
        bytes_left: 512,
    })
}

impl TerminalWriter {
    pub(crate) fn flush(&mut self) {
        let _ = self.uart.flush();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let Some(mut output) = terminal_output() else {
        anova_cortex_m::halt();
    };
    let location = match info.location() {
        Some(location) => write!(
            &mut output,
            "[PANIC] {}:{}:{}: ",
            location.file(),
            location.line(),
            location.column()
        ),
        None => output.write_str("[PANIC] unknown location: "),
    };
    let _ = location.and_then(|()| write!(&mut output, "{}\r\n", info.message()));
    output.flush();
    anova_cortex_m::halt()
}
