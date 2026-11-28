// SPDX-License-Identifier: Apache-2.0
mod runtime;

use anova_board::{Led, usart::Usart1};
use anova_cortex_m::{context, systick};
use anova_kernel::{task::stack::StackStorage, time::TICK_HZ};
use core::fmt::Write;

static mut STACKS: [StackStorage; 4] = [const { StackStorage([0; 256]) }; 4];

pub fn run(_led: Led, mut uart: Option<Usart1>, hclk: u32) -> ! {
    let reload = systick::start(hclk, TICK_HZ).expect("M4 SysTick");
    if let Some(uart) = uart.as_mut() {
        let _ = writeln!(uart, "[M4] HCLK={hclk} reload={reload}\r");
    }
    // SAFETY: Boot runs once and owns these disjoint static stack objects.
    // Runtime takes exclusive loans before the first task can execute.
    let [first, second, third, park] = unsafe { &mut *(&raw mut STACKS) };
    runtime::create_static("first", &mut first.0, worker, 1).unwrap();
    runtime::create_static("second", &mut second.0, worker, 2).unwrap();
    runtime::create_static("third", &mut third.0, worker, 3).unwrap();
    runtime::start(&mut park.0, idle).unwrap();
    anova_cortex_m::halt()
}

extern "C" fn worker(argument: u32) {
    assert!((1..=3).contains(&argument));
    assert!(context::uses_process_stack());
    assert!(runtime::current().unwrap().is_some());
    assert_eq!(
        runtime::state(runtime::current().unwrap().unwrap()),
        Ok(anova_kernel::task::State::Running)
    );
    for _ in 0..333_334 {
        runtime::yield_now().unwrap();
    }
}

extern "C" fn idle(_: u32) {
    assert!(runtime::current().unwrap().is_none());
    assert!(crate::m2::ticks() != 0);
    loop {
        // SAFETY: No ready tasks remain. SysTick still uses MSP.
        unsafe { core::arch::asm!("wfi", options(nostack, preserves_flags)) };
    }
}
