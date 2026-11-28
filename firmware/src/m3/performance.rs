// SPDX-License-Identifier: Apache-2.0
use anova_cortex_m::{cycles::CycleCounter, interrupt};
use anova_kernel::memory::{
    MemoryError,
    pool::MemoryPool,
    tlsf::{FL_COUNT, SL_COUNT, Tlsf},
};
use core::{
    alloc::Layout,
    hint::black_box,
    sync::atomic::{Ordering, compiler_fence},
};

use super::{BoardLog, global};

const SAMPLES: u32 = 128;

struct Timing {
    min: u32,
    max: u32,
    sum: u64,
}

impl Timing {
    const fn new() -> Self {
        Self {
            min: u32::MAX,
            max: 0,
            sum: 0,
        }
    }

    fn time<T>(&mut self, counter: &CycleCounter, operation: impl FnOnce() -> T) -> T {
        let start = counter.read();
        compiler_fence(Ordering::SeqCst);
        let result = black_box(operation());
        compiler_fence(Ordering::SeqCst);
        let cycles = counter.read().wrapping_sub(start);
        self.min = self.min.min(cycles);
        self.max = self.max.max(cycles);
        self.sum += u64::from(cycles);
        result
    }

    fn report(&self, sink: &mut BoardLog<'_>, scenario: &str, name: &str) {
        report!(
            sink,
            "[M3 PERF] {} {} min={} max={} avg={} samples={}",
            scenario,
            name,
            self.min,
            self.max,
            self.sum / u64::from(SAMPLES),
            SAMPLES
        );
    }
}

pub(super) fn run(hclk: u32, pool: &mut MemoryPool<'_>, sink: &mut BoardLog<'_>) {
    let Some(counter) = CycleCounter::probe() else {
        report!(sink, "[M3 PERF] Not Run (DWT CYCCNT unavailable)");
        return;
    };
    assert!(!interrupt::interrupts_enabled());
    let total = global::with(|heap| heap.stats().total).unwrap();
    report!(
        sink,
        "[M3 PERF] HCLK={} opt=s PRIMASK=1 heap={} FL={} SL={} sizes=24/40/64/96B high-align=256B OOM=full-arena/1B",
        hclk,
        total,
        FL_COUNT,
        SL_COUNT
    );
    let mut allocation = Timing::new();
    let mut deallocation = Timing::new();
    let mut mask = Timing::new();
    let mut overhead = Timing::new();
    for _ in 0..SAMPLES {
        let ptr = allocation.time(&counter, || pool.allocate().unwrap());
        // SAFETY: ptr is an exclusively owned live pool block.
        deallocation.time(&counter, || unsafe { pool.deallocate(ptr) }.unwrap());
        mask.time(&counter, || interrupt::critical_section(|| black_box(())));
        overhead.time(&counter, || ());
    }
    allocation.report(sink, "pool", "allocate");
    deallocation.report(sink, "pool", "deallocate");
    mask.report(sink, "baseline", "PRIMASK entry/exit");
    overhead.report(sink, "baseline", "counter envelope");

    let exact_layout = Layout::from_size_align(total - 16, 8).unwrap();
    let mut exact = Timing::new();
    let mut isolated = Timing::new();
    for _ in 0..SAMPLES {
        global::with(|heap| {
            let ptr = exact
                .time(&counter, || heap.allocate(exact_layout))
                .unwrap();
            // SAFETY: ptr is live and no payload is borrowed.
            isolated
                .time(&counter, || unsafe { heap.deallocate(ptr) })
                .unwrap();
        })
        .unwrap();
    }
    exact.report(sink, "unfragmented", "TLSF exact allocate");
    isolated.report(sink, "unfragmented", "TLSF isolated deallocate");

    let mut oom = Timing::new();
    global::with(|heap| {
        let fill = heap.allocate(exact_layout).unwrap();
        for _ in 0..SAMPLES {
            assert_eq!(
                oom.time(&counter, || heap.allocate(Layout::new::<u8>())),
                Err(MemoryError::Exhausted)
            );
        }
        // SAFETY: The exact-fit allocation is still live and unborrowed.
        unsafe { heap.deallocate(fill) }.unwrap();
    })
    .unwrap();
    oom.report(sink, "exhausted", "TLSF OOM");

    for fragmented in [false, true] {
        let layout = Layout::from_size_align(96, 8).unwrap();
        let anchors = if fragmented {
            let blocks =
                [(); 12].map(|_| global::with(|heap| heap.allocate(layout)).unwrap().unwrap());
            for pair in blocks.as_chunks::<2>().0 {
                // SAFETY: Each even-indexed block is freed exactly once here.
                global::with(|heap| unsafe { heap.deallocate(pair[0]) })
                    .unwrap()
                    .unwrap();
            }
            Some(blocks)
        } else {
            None
        };
        let scenario = if fragmented {
            "fragmented"
        } else {
            "unfragmented"
        };
        let mut split = Timing::new();
        let mut coalesce = Timing::new();
        let mut aligned = Timing::new();
        let mut protected = Timing::new();
        let mut protected_free = Timing::new();
        for i in 0..SAMPLES {
            let small = Layout::from_size_align([24, 40, 64, 96][i as usize % 4], 8).unwrap();
            let split_layout = Layout::from_size_align(40, 8).unwrap();
            global::with(|heap| {
                let ptr = split
                    .time(&counter, || heap.allocate(split_layout))
                    .unwrap();
                // SAFETY: ptr is live and no payload is borrowed.
                coalesce
                    .time(&counter, || unsafe { heap.deallocate(ptr) })
                    .unwrap();
                let high = Layout::from_size_align(40, 256).unwrap();
                let ptr = aligned.time(&counter, || heap.allocate(high)).unwrap();
                // SAFETY: Same owner releases the live high-alignment block.
                unsafe { heap.deallocate(ptr) }.unwrap();
            })
            .unwrap();
            let ptr = protected
                .time(&counter, || global::with(|heap| heap.allocate(small)))
                .unwrap()
                .unwrap();
            // SAFETY: Same owner releases this live block before the next sample.
            protected_free
                .time(&counter, || {
                    global::with(|heap| unsafe { heap.deallocate(ptr) })
                })
                .unwrap()
                .unwrap();
        }
        split.report(sink, scenario, "TLSF allocate with split");
        coalesce.report(sink, scenario, "TLSF deallocate with coalesce");
        aligned.report(sink, scenario, "TLSF high alignment");
        protected.report(sink, scenario, "TLSF protected allocate call");
        protected_free.report(sink, scenario, "TLSF protected deallocate call");
        if let Some(anchors) = anchors {
            for pair in anchors.as_chunks::<2>().0 {
                // SAFETY: The odd-indexed anchors remained live through measurement.
                global::with(|heap| unsafe { heap.deallocate(pair[1]) })
                    .unwrap()
                    .unwrap();
            }
        }
        let diagnostics = global::with(|heap: &mut Tlsf<'_>| heap.validate())
            .unwrap()
            .unwrap();
        assert_eq!(diagnostics.largest_free_block, total - 16);
    }
}

pub(super) fn run_unmasked(sink: &mut BoardLog<'_>, ticks: impl Fn() -> u32) {
    let Some(counter) = CycleCounter::probe() else {
        return;
    };
    assert!(interrupt::interrupts_enabled());
    let mut allocation = Timing::new();
    let mut deallocation = Timing::new();
    let mut critical_envelope = Timing::new();
    let before = ticks();
    let layout = Layout::from_size_align(40, 8).unwrap();
    for _ in 0..SAMPLES {
        let ptr = allocation
            .time(&counter, || global::with(|heap| heap.allocate(layout)))
            .unwrap()
            .unwrap();
        // SAFETY: ptr is live, exclusively owned and unborrowed.
        deallocation
            .time(&counter, || {
                global::with(|heap| unsafe { heap.deallocate(ptr) })
            })
            .unwrap()
            .unwrap();
        critical_envelope.time(&counter, || interrupt::critical_section(|| black_box(())));
    }
    let delivered = ticks().wrapping_sub(before);
    report!(
        sink,
        "[M3 PERF] SysTick enabled; ticks delivered during batch={}",
        delivered
    );
    allocation.report(sink, "SysTick-enabled", "TLSF protected allocate call");
    deallocation.report(sink, "SysTick-enabled", "TLSF protected deallocate call");
    critical_envelope.report(sink, "SysTick-enabled", "empty PRIMASK entry/exit");
}
