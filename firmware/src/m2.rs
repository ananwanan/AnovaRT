// SPDX-License-Identifier: Apache-2.0
use crate::m1;
use anova_board::{self as board, irq, rcc::Fault};
use anova_cortex_m::{
    fault::{self, ExceptionFrame, Status},
    interrupt, nvic,
    nvic::{Priority, SystemException},
    systick,
};
use anova_kernel::time::{TICK_HZ, Timebase};
use core::{
    arch::{asm, global_asm},
    fmt::Write,
    sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed},
};

const NVIC_TEST: bool = cfg!(feature = "m2-test-nvic");
const CRITICAL_TEST: bool = cfg!(feature = "m2-test-critical");
const SVC_TEST: bool = cfg!(feature = "m2-test-svc");
const WRAP_TEST: bool = cfg!(feature = "m2-test-wrap");
const USAGE_TEST: bool = cfg!(feature = "m2-fault-usage");
const BUS_TEST: bool = cfg!(feature = "m2-fault-bus");
const HARD_TEST: bool = cfg!(feature = "m2-fault-hard");
const IRQ_TEST: bool = cfg!(feature = "m2-fault-irq");
const SILENT_TEST: bool = cfg!(feature = "m2-fault-silent");

const fn selected(profiles: &[bool]) -> usize {
    let mut index = 0;
    let mut count = 0;
    while index < profiles.len() {
        count += profiles[index] as usize;
        index += 1;
    }
    count
}

const _: () = assert!(
    selected(&[
        NVIC_TEST,
        CRITICAL_TEST,
        SVC_TEST,
        WRAP_TEST,
        USAGE_TEST,
        BUS_TEST,
        HARD_TEST,
        IRQ_TEST,
        SILENT_TEST,
        cfg!(feature = "m1-fault-hse"),
        cfg!(feature = "m1-fault-pll"),
        cfg!(feature = "m1-fault-switch"),
        cfg!(feature = "m1-panic"),
    ]) <= 1,
    "select only one M2 validation profile or inherited M1 test"
);

static CLOCK: Timebase = Timebase::new(if WRAP_TEST { u32::MAX - 500 } else { 0 });
static SVC_SEEN: AtomicU32 = AtomicU32::new(0);
static PENDSV_SEEN: AtomicU32 = AtomicU32::new(0);
static TRACE: AtomicU32 = AtomicU32::new(0);
static TRACE_INDEX: AtomicU32 = AtomicU32::new(0);
static IRQ_VALID: AtomicBool = AtomicBool::new(true);
static HIGH_PENDS_LOW: AtomicBool = AtomicBool::new(false);

const LOW: irq::Irq = irq::DMA1_CHANNEL1;
const HIGH: irq::Irq = irq::DMA1_CHANNEL2;

const fn priority(level: u8) -> Priority {
    match Priority::new(level) {
        Some(priority) => priority,
        None => panic!("invalid priority"),
    }
}

pub fn run(mut led: board::Led) -> ! {
    let fault = if cfg!(feature = "m1-fault-hse") {
        Fault::HseTimeout
    } else if cfg!(feature = "m1-fault-pll") {
        Fault::PllTimeout
    } else if cfg!(feature = "m1-fault-switch") {
        Fault::SwitchTimeout
    } else {
        Fault::None
    };
    let (init, mut serial) = m1::initialize(fault, !SILENT_TEST);
    nvic::configure_preemption();
    fault::configure();
    SystemException::SvCall.set_priority(priority(8));
    SystemException::SysTick.set_priority(priority(14));
    SystemException::PendSv.set_priority(priority(15));

    if let Some(uart) = serial.as_mut() {
        let _ = writeln!(uart, "[M2] Interrupts and exceptions ready\r");
    }
    if cfg!(feature = "m1-panic") {
        panic!("M1 panic diagnostic test");
    }

    // SAFETY: VTOR and the handlers are installed. SysTick and external IRQs
    // are enabled only by the code that owns their state.
    unsafe { interrupt::unmask() };
    if NVIC_TEST {
        test_nvic();
        report(
            &mut serial,
            "NVIC",
            "low/high/low=0x312 high/high/low/low=0x3241",
        );
    }
    if CRITICAL_TEST {
        test_critical();
        report(&mut serial, "critical", "PRIMASK BASEPRI IPSR");
    }
    if SVC_TEST {
        test_svc();
        report(&mut serial, "SVC/PendSV", "returned to Thread Mode");
    }
    if IRQ_TEST {
        irq::DMA1_CHANNEL3.enable();
        irq::DMA1_CHANNEL3.set_pending();
        panic!("unexpected IRQ injection did not dispatch");
    }
    if BUS_TEST {
        // SAFETY: This validation profile deliberately tests a reserved bus
        // address with one read, without constructing a Rust reference.
        unsafe {
            asm!("ldr {value}, [{address}]",
                address = in(reg) 0x400f_0000u32,
                value = out(reg) _, options(nostack, preserves_flags));
        }
        panic!("BusFault probe returned without a fault");
    }
    if HARD_TEST {
        fault::disable_usage_fault();
    }
    if USAGE_TEST || HARD_TEST || SILENT_TEST {
        // SAFETY: UDF intentionally enters a terminal exception handler.
        unsafe { asm!("udf #0", options(noreturn, nostack)) };
    }

    let mut edge = CLOCK.ticks();
    let mut log = edge;
    let mut previous_tick = edge;
    let reload = systick::start(init.clocks.hclk_hz, TICK_HZ).expect("invalid SysTick clock");
    if let Some(uart) = serial.as_mut() {
        let _ = writeln!(
            uart,
            "[M2] SysTick HCLK={} RELOAD={} Hz={}\r",
            init.clocks.hclk_hz, reload, TICK_HZ
        );
    }
    let mut on = false;
    loop {
        let now = CLOCK.uptime_ms();
        if WRAP_TEST && now < previous_tick {
            assert!(now.wrapping_sub(previous_tick) < 500);
            report(&mut serial, "tick wrap", "tick advanced across u32::MAX");
        }
        previous_tick = now;
        let periods = now.wrapping_sub(edge) / 500;
        if periods != 0 {
            edge = edge.wrapping_add(periods * 500);
            on ^= periods & 1 != 0;
            led.set(on);
        }
        if now.wrapping_sub(log) >= 1000 {
            log = now;
            if let Some(uart) = serial.as_mut() {
                let _ = writeln!(uart, "[M2] ticks={}\r", now);
            }
        }
        m1::echo(&mut serial);
        // SAFETY: No resource is borrowed by an ISR. The next tick also
        // handles the race where a tick arrives immediately before WFI.
        unsafe { asm!("wfi", options(nostack, preserves_flags)) };
    }
}

fn report(serial: &mut Option<board::usart::Usart1>, test: &str, details: &str) {
    if let Some(uart) = serial.as_mut() {
        let _ = writeln!(uart, "[M2 PASS] {test}: {details}\r");
    }
}

extern "C" fn systick_entry() {
    CLOCK.tick();
}

extern "C" fn svc_entry() {
    SVC_SEEN.store(u32::from(interrupt::exception_number()), Relaxed);
}

extern "C" fn pendsv_entry() {
    PENDSV_SEEN.store(u32::from(interrupt::exception_number()), Relaxed);
}

fn test_svc() {
    assert!(!interrupt::in_interrupt_context());
    // SAFETY: Thread Mode is privileged, unmasked, and the SVC vector returns.
    unsafe { asm!("svc #0", options(nostack)) };
    assert_eq!(SVC_SEEN.load(Relaxed), 11);
    nvic::pend_pendsv();
    assert_eq!(PENDSV_SEEN.load(Relaxed), 14);
    assert!(!interrupt::in_interrupt_context());
}

fn event(code: u32) {
    let index = TRACE_INDEX.fetch_add(1, Relaxed);
    if index < 8 {
        TRACE.fetch_or(code << (4 * index), Relaxed);
    } else {
        IRQ_VALID.store(false, Relaxed);
    }
}

extern "C" fn low_handler() {
    if !LOW.is_active() || HIGH.is_active() || interrupt::exception_number() != 27 {
        IRQ_VALID.store(false, Relaxed);
    }
    event(2);
    if !HIGH_PENDS_LOW.load(Relaxed) {
        HIGH.set_pending();
    }
    event(3);
}

extern "C" fn high_handler() {
    let high_started_first = HIGH_PENDS_LOW.load(Relaxed);
    if !HIGH.is_active()
        || LOW.is_active() == high_started_first
        || interrupt::exception_number() != 28
    {
        IRQ_VALID.store(false, Relaxed);
    }
    event(1);
    if high_started_first {
        LOW.set_pending();
        if !LOW.is_pending() || LOW.is_active() {
            IRQ_VALID.store(false, Relaxed);
        }
        event(4);
    }
}

global_asm!(
    include_str!("m2_entries.s"),
    test_irq = const (NVIC_TEST || CRITICAL_TEST) as u8,
    low = const LOW.number(),
    high = const HIGH.number(),
    low_handler = sym low_handler,
    high_handler = sym high_handler,
    systick = sym systick_entry,
    svc = sym svc_entry,
    pendsv = sym pendsv_entry,
);

fn setup_test_irqs() {
    LOW.set_priority(priority(9));
    HIGH.set_priority(priority(3));
    assert_eq!(nvic::priority_grouping(), 3);
    assert_eq!(LOW.priority(), priority(9));
    assert_eq!(HIGH.priority(), priority(3));
    LOW.disable();
    HIGH.enable();
}

fn finish_test_irqs() {
    LOW.disable();
    HIGH.disable();
    LOW.clear_pending();
    HIGH.clear_pending();
    assert!(!LOW.is_enabled() && !HIGH.is_enabled());
    assert!(!LOW.is_pending() && !HIGH.is_pending());
    assert!(!LOW.is_active() && !HIGH.is_active());
    assert!(IRQ_VALID.load(Relaxed));
}

fn check_trace(expected: u32, events: u32) {
    assert_eq!(TRACE.swap(0, Relaxed), expected);
    assert_eq!(TRACE_INDEX.swap(0, Relaxed), events);
}

fn test_nvic() {
    setup_test_irqs();
    LOW.set_pending();
    assert!(LOW.is_pending() && !LOW.is_enabled() && !LOW.is_active());
    check_trace(0, 0);
    LOW.clear_pending();
    assert!(!LOW.is_pending());
    interrupt::critical_section(|| {
        LOW.enable();
        LOW.set_pending();
        assert!(LOW.is_enabled() && LOW.is_pending() && !LOW.is_active());
        check_trace(0, 0);
    });
    check_trace(0x312, 3);
    HIGH_PENDS_LOW.store(true, Relaxed);
    HIGH.set_pending();
    check_trace(0x3241, 4);
    finish_test_irqs();
}

fn test_critical() {
    setup_test_irqs();
    LOW.enable();
    assert!(interrupt::interrupts_enabled());
    interrupt::critical_section(|| {
        assert!(!interrupt::interrupts_enabled());
        interrupt::critical_section(|| assert!(!interrupt::interrupts_enabled()));
        assert!(!interrupt::interrupts_enabled());
        LOW.set_pending();
        assert!(LOW.is_pending());
        check_trace(0, 0);
    });
    assert!(interrupt::interrupts_enabled());
    check_trace(0x312, 3);

    HIGH_PENDS_LOW.store(true, Relaxed);
    interrupt::with_priority_ceiling(priority(5), || {
        assert_eq!(interrupt::basepri(), 5);
        interrupt::with_priority_ceiling(priority(10), || assert_eq!(interrupt::basepri(), 5));
        interrupt::with_priority_ceiling(priority(2), || assert_eq!(interrupt::basepri(), 2));
        interrupt::with_priority_ceiling(priority(0), || assert_eq!(interrupt::basepri(), 5));
        assert_eq!(interrupt::basepri(), 5);
        HIGH.set_pending();
        assert!(LOW.is_pending());
        check_trace(0x41, 2);
    });
    assert_eq!(interrupt::basepri(), 0);
    check_trace(0x32, 2);
    finish_test_irqs();
}

unsafe extern "C" {
    static __sram_start: u8;
    static __sram_end: u8;
    static __fault_stack_bottom: u8;
    static __fault_stack_top: u8;
}

#[repr(C)]
struct FaultRecord {
    exception: u32,
    exc_return: u32,
    msp: u32,
    psp: u32,
    status: Status,
    frame_valid: u32,
    frame: ExceptionFrame,
    // Written last so a debugger can distinguish a complete snapshot.
    complete: u32,
}

#[unsafe(no_mangle)]
static mut M2_FAULT_RECORD: core::mem::MaybeUninit<FaultRecord> = core::mem::MaybeUninit::zeroed();

#[unsafe(no_mangle)]
extern "C" fn m2_fault(exception: u32, exc_return: u32, msp: u32, psp: u32) -> ! {
    let start = &raw const __sram_start as u32;
    let end = &raw const __sram_end as u32;
    let emergency_start = &raw const __fault_stack_bottom as u32;
    let emergency_end = &raw const __fault_stack_top as u32;
    let status = Status::read();
    let frame = fault::frame_address(exc_return, msp, psp, status.cfsr, start, end)
        .filter(|&address| address + 36 <= emergency_start || address >= emergency_end)
        .and_then(|address| {
            // SAFETY: Checked aligned, initialized hardware frame within SRAM.
            // The diagnostic stack cannot overwrite it. Volatile reads allow
            // capture of corrupted frames as plain integers.
            let frame = unsafe { core::ptr::read_volatile(address as *const ExceptionFrame) };
            fault::valid_padding(address, frame.xpsr, start, end).then_some(frame)
        });
    let record = FaultRecord {
        exception,
        exc_return,
        msp,
        psp,
        status,
        frame_valid: u32::from(frame.is_some()),
        frame: frame.unwrap_or_default(),
        complete: 0,
    };
    let destination = (&raw mut M2_FAULT_RECORD).cast::<FaultRecord>();
    // SAFETY: The terminal bridge masks configurable exceptions and refuses
    // reentry on its emergency stack. The aligned record is owned here.
    unsafe {
        destination.write_volatile(record);
        (&raw mut (*destination).complete).write_volatile(0x4d32_4641);
    }
    interrupt::dsb();

    if let Some(mut output) = m1::terminal_output() {
        let _ = write_fault(&mut output, exception, exc_return, msp, psp, status, frame);
        output.flush();
    }
    anova_cortex_m::halt()
}

fn write_fault(
    out: &mut impl Write,
    exception: u32,
    exc_return: u32,
    msp: u32,
    psp: u32,
    status: Status,
    frame: Option<ExceptionFrame>,
) -> core::fmt::Result {
    let label = match exception {
        2 => "NMI",
        3 => "HardFault",
        4 => "MemManage",
        5 => "BusFault",
        6 => "UsageFault",
        16..=75 => "Unexpected IRQ",
        _ => "Unexpected exception",
    };
    writeln!(
        out,
        "[FATAL] {label} IPSR={exception} EXC_RETURN={exc_return:#010x}\r"
    )?;
    writeln!(out, "[FAULT] MSP={msp:#010x} PSP={psp:#010x}\r")?;
    if let Some(frame) = frame {
        writeln!(
            out,
            "[FAULT] PC={:#010x} LR={:#010x} xPSR={:#010x}\r",
            frame.pc, frame.lr, frame.xpsr
        )?;
    } else {
        writeln!(out, "[FAULT] frame unavailable\r")?;
    }
    writeln!(
        out,
        "[FAULT] CFSR={:#010x} HFSR={:#010x} DFSR={:#010x} AFSR={:#010x}\r",
        status.cfsr, status.hfsr, status.dfsr, status.afsr
    )?;
    writeln!(
        out,
        "[FAULT] SHCSR={:#010x} ICSR={:#010x}\r",
        status.shcsr, status.icsr
    )?;
    if let Some(address) = status.memory_address() {
        writeln!(out, "[FAULT] MMFAR={address:#010x}\r")?;
    }
    if let Some(address) = status.bus_address() {
        writeln!(out, "[FAULT] BFAR={address:#010x}\r")?;
    }
    for reason in status.reasons() {
        writeln!(out, "[FAULT] {reason}\r")?;
    }
    writeln!(out, "[FAULT] System halted\r")
}
