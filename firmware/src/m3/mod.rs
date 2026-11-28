// SPDX-License-Identifier: Apache-2.0
use anova_board::{memory, usart::Usart1};
use anova_kernel::log::{Level, Sink, TextSink};
use anova_kernel::memory::{
    MemoryError,
    pool::{MemoryPool, PoolStats},
    stack::{self, StackRegion, StackStats},
};
use core::{fmt, hint::black_box};

struct BoardLog<'a>(Option<&'a mut Usart1>);

impl Sink for BoardLog<'_> {
    fn write_record(&mut self, level: Level, message: fmt::Arguments<'_>) -> fmt::Result {
        if let Some(uart) = self.0.as_deref_mut() {
            TextSink(uart).write_record(level, message)
        } else {
            Ok(())
        }
    }
}

macro_rules! report {
    ($sink:expr, $($argument:tt)+) => {
        let _ = anova_kernel::info!($sink, $($argument)+);
    };
}

#[cfg(feature = "heap-tlsf")]
mod global;
#[cfg(feature = "heap-tlsf")]
mod heap_tests;
#[cfg(feature = "m3-test-performance")]
mod performance;

const POOL: bool = cfg!(feature = "m3-test-pool");
const TLSF: bool = cfg!(feature = "m3-test-tlsf");
const ALIGNMENT: bool = cfg!(feature = "m3-test-alignment");
const FRAGMENTATION: bool = cfg!(feature = "m3-test-fragmentation");
const OOM: bool = cfg!(feature = "m3-test-oom");
const STACK: bool = cfg!(feature = "m3-test-stack");
const CORRUPTION: bool = cfg!(feature = "m3-test-corruption");
const PERFORMANCE: bool = cfg!(feature = "m3-test-performance");
const _: () = assert!(
    POOL as u8
        + TLSF as u8
        + ALIGNMENT as u8
        + FRAGMENTATION as u8
        + OOM as u8
        + STACK as u8
        + CORRUPTION as u8
        + PERFORMANCE as u8
        <= 1,
    "select only one M3 validation profile"
);

struct MemoryStats {
    sram: usize,
    static_ram: usize,
    pool_reserved: usize,
    pool: PoolStats,
    stack: StackStats,
    #[cfg(feature = "heap-tlsf")]
    heap: anova_kernel::memory::tlsf::HeapStats,
}

#[inline(never)]
pub fn initialize(hclk: u32, serial: &mut Option<Usart1>) -> MemoryPool<'static> {
    let regions = memory::regions();
    // SAFETY: M2 hands off here once, before configurable exceptions dispatch.
    let mut pool = MemoryPool::new(unsafe { memory::pool_backing() }, 32).expect("pool arena");
    #[cfg(feature = "heap-tlsf")]
    global::initialize();
    let mut sink = BoardLog(serial.as_mut());

    pool_check(&mut pool, if POOL { pool_capacity(&regions) } else { 1 });
    report!(&mut sink, "[M3 PASS] pool");
    #[cfg(feature = "heap-tlsf")]
    heap_tests::run(&mut sink);
    let measured_stack = measure_stack(&regions);
    assert!(measured_stack.canary_intact && measured_stack.pointer_valid);
    if STACK || CORRUPTION {
        let (shallow, deep) = stack_check(&regions);
        report!(
            &mut sink,
            "[M3 PASS] Stack depth shallow={} deep={} controlled canary corruption detected",
            shallow,
            deep
        );
    }
    report!(&mut sink, "[M3 PASS] MSP watermark and canary");

    #[cfg(feature = "m3-test-performance")]
    performance::run(hclk, &mut pool, &mut sink);
    let stats = MemoryStats {
        sram: regions.sram.len(),
        static_ram: regions.static_ram.len(),
        pool_reserved: regions.pool.len(),
        pool: pool.stats(),
        stack: measure_stack(&regions),
        #[cfg(feature = "heap-tlsf")]
        heap: global::with(|heap| heap.stats()).expect("initialized heap"),
    };
    stats.report(hclk, &mut sink);
    report!(&mut sink, "[M3 PASS] memory self-test");
    pool
}

fn pool_capacity(regions: &memory::Regions) -> usize {
    regions.pool.len() / 32
}

fn pool_check(pool: &mut MemoryPool<'_>, count: usize) {
    let mut active = [None; 64];
    for slot in &mut active[..count] {
        let address = pool.allocate().expect("pool allocation");
        assert_eq!(address.addr().get() % 32, 0);
        assert_eq!(pool.address(pool.index(address).unwrap()), Some(address));
        *slot = Some(address);
    }
    if count == pool.stats().capacity {
        assert_eq!(pool.allocate(), Err(MemoryError::Exhausted));
    }
    for address in active[..count].iter().flatten().rev() {
        // SAFETY: Each slot holds a different live block. No payload is borrowed.
        unsafe { pool.deallocate(*address) }.expect("pool release");
    }
    let first = active[0].unwrap();
    // SAFETY: This known free address tests the bitmap rejection, with no live owner.
    assert_eq!(
        unsafe { pool.deallocate(first) },
        Err(MemoryError::DoubleFree)
    );
    assert_eq!(pool.allocate().unwrap(), first);
    // SAFETY: The previous allocation returned first and it is now live.
    unsafe { pool.deallocate(first) }.unwrap();
    assert_eq!(pool.stats().used, 0);
    assert_eq!(pool.stats().peak_used, count);
}

fn measure_stack(regions: &memory::Regions) -> StackStats {
    let bottom = regions.main_stack.start;
    let top = regions.main_stack.end;
    let region = StackRegion::new(bottom, top).expect("MSP bounds");
    let sample = anova_cortex_m::interrupt::critical_section(|| {
        // SAFETY: Reset initialized MSP. Thread Mode uses MSP, configurable
        // interrupts are masked, and non-maskable handlers are terminal.
        unsafe { anova_cortex_m::stack::sample(bottom, top, stack::PATTERN, stack::CANARY) }
    });
    region.summarize(sample.sp, sample.first_used, sample.canary_intact)
}

#[inline(never)]
fn deep_sample(regions: &memory::Regions) -> StackStats {
    let mut frame = [0x1234_5678u32; 192];
    black_box(&mut frame);
    let stats = measure_stack(regions);
    black_box(&frame);
    stats
}

fn stack_check(regions: &memory::Regions) -> (usize, usize) {
    let shallow = measure_stack(regions);
    let deep = deep_sample(regions);
    assert!(deep.canary_intact && deep.pointer_valid);
    assert!(deep.stack_pointer < shallow.stack_pointer);
    assert!(deep.peak_used >= shallow.peak_used);
    let mut storage = [0u64; 8];
    // SAFETY: u64 storage is aligned and initialized, and all bit patterns
    // are valid for both types. This uniquely borrows the same 64 bytes.
    let words = unsafe { core::slice::from_raw_parts_mut(storage.as_mut_ptr().cast::<u32>(), 16) };
    let simulated = stack::fill_stack(words).unwrap();
    let top = words.as_ptr() as usize + core::mem::size_of_val(words);
    assert_eq!(simulated.inspect(words, top).unwrap().remaining, 56);
    words[7] = 0;
    assert_eq!(simulated.inspect(words, top).unwrap().remaining, 20);
    words[0] = 0;
    assert!(!simulated.inspect(words, top).unwrap().canary_intact);
    assert!(!simulated.inspect(words, top + 8).unwrap().pointer_valid);
    (shallow.stack_pointer, deep.stack_pointer)
}

impl MemoryStats {
    fn report(self, hclk: u32, sink: &mut BoardLog<'_>) {
        report!(
            sink,
            "[M3] HCLK={} SRAM={} static={} pool reserved={} used={} free={} peak={}",
            hclk,
            self.sram,
            self.static_ram,
            self.pool_reserved,
            self.pool.used * self.pool.block_size,
            (self.pool.capacity - self.pool.used) * self.pool.block_size,
            self.pool.peak_used * self.pool.block_size
        );
        report!(
            sink,
            "[M3] MSP size={} sp={:#010x} remaining={} peak={} canary={}",
            self.stack.size,
            self.stack.stack_pointer,
            self.stack.remaining,
            self.stack.peak_used,
            self.stack.canary_intact
        );
        #[cfg(feature = "heap-tlsf")]
        {
            use anova_kernel::memory::tlsf::{FL_COUNT, SL_COUNT};
            let h = self.heap;
            report!(
                sink,
                "[M3] TLSF FL={} SL={} total={} used={} requested={} free={} peak={} metadata={} padding={} blocks={}/{} alloc={} free_calls={} failed={}",
                FL_COUNT,
                SL_COUNT,
                h.total,
                h.used,
                h.requested,
                h.free(),
                h.peak_used,
                h.metadata(),
                h.internal_padding(),
                h.allocated_blocks,
                h.free_blocks,
                h.allocations,
                h.deallocations,
                h.failures
            );
        }
        #[cfg(not(feature = "heap-tlsf"))]
        report!(sink, "[M3] TLSF disabled");
    }
}

#[cfg(feature = "m3-test-tlsf")]
pub fn test_handler_rejection(serial: &mut Option<Usart1>) {
    let before = global::handler_rejections();
    // SAFETY: M2 has enabled configurable exceptions and installed a returning SVC.
    unsafe { core::arch::asm!("svc #0", options(nostack)) };
    assert_eq!(global::handler_rejections().wrapping_sub(before), 1);
    report!(
        &mut BoardLog(serial.as_mut()),
        "[M3 PASS] Handler Mode allocation rejected"
    );
}

#[cfg(feature = "m3-test-tlsf")]
pub fn request_in_handler() {
    global::request_in_handler();
}

#[cfg(feature = "m3-test-performance")]
pub fn test_with_systick(serial: &mut Option<Usart1>, ticks: impl Fn() -> u32) {
    performance::run_unmasked(&mut BoardLog(serial.as_mut()), ticks);
}
