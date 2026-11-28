// SPDX-License-Identifier: Apache-2.0
mod runtime;

use anova_board::{Led, irq, memory, usart::Usart1};
use anova_cortex_m::{
    context::{self, timing},
    cycles::CycleCounter,
    interrupt,
    nvic::Priority,
    systick,
};
use anova_kernel::{
    task::{
        Error as TableError, State, TaskId,
        stack::{Stack, StackStorage},
    },
    time::TICK_HZ,
};
use core::{
    fmt::Write,
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed},
};

const ROUNDS: u32 = if cfg!(feature = "m4-test-smoke") {
    1024
} else {
    333_334
};
const IRQ: irq::Irq = irq::DMA1_CHANNEL2;
static IRQ_COUNT: AtomicU32 = AtomicU32::new(0);
static DWT: AtomicBool = AtomicBool::new(false);
static FULL_START: AtomicU32 = AtomicU32::new(0);
static FULL_ARMED: AtomicBool = AtomicBool::new(false);

static mut STACKS: [StackStorage; 3] = [const { StackStorage([0; 256]) }; 3];
static mut PARK: StackStorage<512> = StackStorage([0; 512]);

struct Board {
    led: Led,
    uart: Option<Usart1>,
    clock: u32,
    tick_start: u32,
    _counter: Option<CycleCounter>,
}

static mut BOARD: Option<Board> = None;
static mut IDS: Option<[TaskId; 3]> = None;
static mut WORKER_PEAKS: [usize; 2] = [0; 2];
static mut METRICS: [Metric; 4] = [Metric::EMPTY; 4];

#[derive(Clone, Copy)]
struct Metric {
    min: u32,
    max: u32,
    sum: u64,
    count: u32,
}

impl Metric {
    const EMPTY: Self = Self {
        min: u32::MAX,
        max: 0,
        sum: 0,
        count: 0,
    };

    fn add(&mut self, cycles: u32) {
        self.min = self.min.min(cycles);
        self.max = self.max.max(cycles);
        self.sum += u64::from(cycles);
        self.count += 1;
    }
}

core::arch::global_asm!(include_str!("probe.s"), options(raw));

unsafe extern "C" {
    fn m4_probe_registers(seed: u32) -> u32;
}

pub fn run(led: Led, mut uart: Option<Usart1>, hclk: u32) -> ! {
    let reload = systick::start(hclk, TICK_HZ).expect("M4 SysTick");
    let counter = CycleCounter::probe();
    if let Some(counter) = counter.as_ref() {
        timing::enable(counter);
        DWT.store(true, Relaxed);
    }
    if let Some(uart) = uart.as_mut() {
        let _ = writeln!(
            uart,
            "[M4] HCLK={hclk} reload={reload} rounds={ROUNDS} TLSF={} DWT={} TCB={} table={}\r",
            cfg!(feature = "heap-tlsf"),
            DWT.load(Relaxed),
            runtime::TCB_BYTES,
            runtime::TABLE_BYTES,
        );
    }

    // SAFETY: Boot runs once and owns these disjoint static stack objects.
    // The runtime takes exclusive loans before the first task can execute.
    let storage = &raw mut STACKS;
    let [first, second, third] = unsafe { &mut *storage };
    // SAFETY: PARK is distinct from the three worker allocations. Failed
    // startup returns the exclusive loan. No other code acquires PARK.
    let park = &raw mut PARK;
    let park = unsafe { &mut (*park).0 };
    let (error, park) = runtime::start(park, idle).unwrap_err();
    assert_eq!(error, runtime::Error::NotStarted);
    let ids = [
        runtime::create_static("first", &mut first.0, worker, 1).unwrap(),
        runtime::create_static("second", &mut second.0, worker, 2).unwrap(),
        runtime::create_static("controller", &mut third.0, worker, 3).unwrap(),
    ];
    for &id in &ids {
        let usage = runtime::usage(id).unwrap();
        assert!(usage.canary_intact && usage.pointer_valid);
        assert_eq!(usage.peak_used, context::FRAME_BYTES);
    }
    // SAFETY: Boot is the only writer. These values are fully initialized
    // before SVC. Only workers read IDS, and only idle takes BOARD.
    unsafe {
        (&raw mut IDS).write(Some(ids));
        (&raw mut BOARD).write(Some(Board {
            led,
            uart,
            clock: hclk,
            tick_start: crate::m2::ticks(),
            _counter: counter,
        }));
    }
    IRQ.clear_pending();
    IRQ.set_priority(Priority::new(3).unwrap());
    IRQ.enable();
    runtime::start(park, idle).unwrap();
    anova_cortex_m::halt()
}

pub fn pend_test_irq() {
    IRQ.set_pending();
    interrupt::synchronize();
}

#[unsafe(no_mangle)]
extern "C" fn m4_irq() {
    assert!(IRQ.is_active());
    assert_eq!(interrupt::exception_number(), 16 + u16::from(IRQ.number()));
    assert!(
        memory::regions()
            .main_stack
            .contains(&context::main_stack_pointer())
    );
    assert_eq!(runtime::yield_now(), Err(runtime::Error::Caller));
    IRQ_COUNT.fetch_add(1, Relaxed);
}

fn ids() -> [TaskId; 3] {
    // SAFETY: SVC runs after the only write. No mutable aliases exist.
    unsafe { (&raw const IDS).read().unwrap() }
}

extern "C" fn worker(argument: u32) {
    record_incoming();
    assert!((1..=3).contains(&argument));
    let ids = ids();
    let index = (argument - 1) as usize;
    let id = ids[index];
    assert!(context::uses_process_stack());
    assert_ne!(
        context::process_stack_pointer(),
        context::main_stack_pointer()
    );
    assert_eq!(runtime::current(), Ok(Some(id)));
    assert_eq!(runtime::state(id), Ok(State::Running));
    assert_eq!(
        runtime::usage(id),
        Err(runtime::Error::Table(TableError::InvalidState))
    );
    assert!(matches!(
        runtime::reap(id),
        Err(runtime::Error::Table(TableError::InvalidState))
    ));
    interrupt::critical_section(|| assert_eq!(runtime::yield_now(), Err(runtime::Error::Caller)));
    interrupt::with_priority_ceiling(Priority::new(15).unwrap(), || {
        assert_eq!(runtime::yield_now(), Err(runtime::Error::Caller));
    });

    #[cfg(feature = "m4-test-invalid")]
    if argument == 1 {
        runtime::inject_invalid_sp(ids[1]);
        runtime::yield_now().unwrap();
        panic!("M4 invalid context unexpectedly resumed");
    }

    for iteration in 0..ROUNDS {
        assert!(nested(3, (argument << 24) | iteration));
        assert_eq!(runtime::current(), Ok(Some(id)));
    }
    if argument == 3 {
        recycle(ids);
    }
}

#[inline(never)]
fn nested(depth: u32, seed: u32) -> bool {
    let marker = black_box([seed, !seed, seed.rotate_left(depth)]);
    let registers_ok = if depth == 0 {
        if DWT.load(Relaxed) {
            FULL_START.store(timing::time(), Relaxed);
            FULL_ARMED.store(true, Relaxed);
        }
        // SAFETY: This runs only inside an unmasked worker on its PSP. The
        // assembly obeys AAPCS, pends PendSV and checks its own saved frame.
        let result = unsafe { m4_probe_registers(seed) };
        record_incoming();
        result == 1
    } else {
        nested(depth - 1, seed)
    };
    let [first, second, third] = black_box(marker);
    registers_ok && first == seed && second == !seed && third == seed.rotate_left(depth)
}

fn record_incoming() {
    if FULL_ARMED.swap(false, Relaxed) {
        let full = timing::time().wrapping_sub(FULL_START.load(Relaxed));
        let [save, restore, software] = timing::samples();
        // SAFETY: Only task Thread Mode updates this array, between yield
        // requests. No reference to it lives across exception switching.
        unsafe {
            let metrics = &raw mut METRICS;
            for (metric, measurement) in (&mut *metrics)
                .iter_mut()
                .zip([save, restore, software, full])
            {
                metric.add(measurement);
            }
        }
    }
}

fn checked_peak(id: TaskId) -> usize {
    let stats = runtime::usage(id).unwrap();
    assert!(stats.pointer_valid && stats.canary_intact && stats.peak_used > context::FRAME_BYTES);
    stats.peak_used
}

fn recycle(ids: [TaskId; 3]) {
    for (index, &id) in ids[..2].iter().enumerate() {
        assert_eq!(runtime::state(id), Ok(State::Terminated));
        // SAFETY: This is the sole writer and idle only reads after exit.
        unsafe {
            (&raw mut WORKER_PEAKS)
                .cast::<usize>()
                .add(index)
                .write(checked_peak(id))
        };
    }

    let saved = runtime::SWITCHES.load(Relaxed);
    runtime::yield_now().unwrap();
    assert_eq!(runtime::SWITCHES.load(Relaxed), saved);

    let words = runtime::reap(ids[0]).unwrap();
    assert!(matches!(
        runtime::reap(ids[0]),
        Err(runtime::Error::Table(TableError::UnknownTask))
    ));
    let (error, words) = runtime::start(words, idle).unwrap_err();
    assert_eq!(error, runtime::Error::Started);
    let (error, _) = runtime::create_static("undersized", &mut words[..2], worker, 0).unwrap_err();
    assert_eq!(error, runtime::Error::InvalidStack);
    let words = runtime::reap(ids[1]).unwrap();
    let stack = Stack::new(words).unwrap();
    assert!(stack.canary_intact());
    // SAFETY: The reaped storage is exclusively owned and has no live
    // frames. This controlled corruption touches just one guard word.
    unsafe { stack.pointer(stack.top() - 1024).write_volatile(0) };
    assert!(!stack.canary_intact());
    // SAFETY: This diagnostic loan has never been scheduled.
    let words = unsafe { stack.into_storage() };
    let replacement = runtime::create_static("reused", words, reused, 0xcafe).unwrap();
    assert_ne!(replacement, ids[1]);
    assert_eq!(
        runtime::state(ids[1]),
        Err(runtime::Error::Table(TableError::UnknownTask))
    );
    runtime::yield_now().unwrap();
    assert_eq!(runtime::state(replacement), Ok(State::Terminated));
    let _returned_storage = runtime::reap(replacement).unwrap();
}

extern "C" fn reused(argument: u32) {
    assert_eq!(argument, 0xcafe);
    assert!(context::uses_process_stack());
    black_box([argument; 16]);
}

extern "C" fn idle(_: u32) {
    assert!(runtime::current().unwrap().is_none());
    assert!(context::uses_process_stack());
    assert_eq!(runtime::yield_now(), Err(runtime::Error::NotStarted));
    let controller = ids()[2];
    let peak = checked_peak(controller);
    let _returned_storage = runtime::reap(controller).unwrap();
    let switches = runtime::SWITCHES.load(Relaxed);
    let irqs = IRQ_COUNT.load(Relaxed);
    assert!(switches == ROUNDS * 3 + 5 && irqs == switches / 256);
    IRQ.disable();
    // SAFETY: Idle is the sole reader of BOARD and the last running context.
    let board_ptr = &raw mut BOARD;
    let mut board = unsafe { (&mut *board_ptr).take().unwrap() };
    let ticks = crate::m2::ticks().wrapping_sub(board.tick_start);
    assert_ne!(ticks, 0);
    // SAFETY: No worker remains to modify the completed diagnostics.
    let [first, second] = unsafe { (&raw const WORKER_PEAKS).read() };
    // SAFETY: The sampling workers have exited. Copy their completed results.
    let metrics = unsafe { (&raw const METRICS).read() };
    if DWT.load(Relaxed) {
        assert!(metrics.iter().all(|metric| metric.count == ROUNDS * 3));
    }
    if let Some(uart) = board.uart.as_mut() {
        let _ = writeln!(
            uart,
            "[M4 PASS] startup PSP/MSP register frames nesting lifecycle reuse canary idle\r"
        );
        let _ = writeln!(
            uart,
            "[M4 PASS] switches={switches} ticks={ticks} nested_irq={irqs} peaks=[{first},{second},{peak}] bytes\r"
        );
        if DWT.load(Relaxed) {
            for (label, metric) in ["save", "restore", "PendSV", "handoff"]
                .into_iter()
                .zip(metrics)
            {
                let avg = metric.sum / u64::from(metric.count);
                let _ = writeln!(
                    uart,
                    "[M4 PERF] {label} HCLK={} min={} max={} avg={avg} n={} IRQ=enabled opt=s\r",
                    board.clock, metric.min, metric.max, metric.count
                );
            }
        } else {
            let _ = writeln!(uart, "[M4] DWT unavailable; performance Not Run\r");
        }
    }

    M4_COMPLETE.store(0x4d34_5041, Relaxed);
    let mut edge = crate::m2::ticks();
    let mut on = true;
    board.led.set(on);
    loop {
        let now = crate::m2::ticks();
        if now.wrapping_sub(edge) >= 500 {
            edge = now;
            on = !on;
            board.led.set(on);
        }
        crate::m1::echo(&mut board.uart);
        // SAFETY: No ready tasks remain. SysTick still uses MSP and wakes WFI.
        unsafe { core::arch::asm!("wfi", options(nostack, preserves_flags)) };
    }
}

#[unsafe(no_mangle)]
static M4_COMPLETE: AtomicU32 = AtomicU32::new(0);
