// SPDX-License-Identifier: Apache-2.0
//! Single-core M4 task runtime. ISRs may never call its Thread APIs.

use anova_cortex_m::{context, interrupt, nvic};
use anova_kernel::task::{Error as TableError, State, Table, TaskId, stack::Stack};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU32, Ordering::Relaxed},
};

pub const CAPACITY: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Caller,
    NotStarted,
    Started,
    InvalidStack,
    Table(TableError),
}

struct Context {
    stack: Stack,
    sp: usize,
    _entry: context::Entry,
    _argument: u32,
}

impl Context {
    fn new(stack: Stack, entry: context::Entry, argument: u32) -> Self {
        let sp = stack.top() - context::FRAME_BYTES;
        let frame =
            context::InitialFrame::new(entry as usize as u32, exit as *const () as u32, argument)
                .expect("Thumb function pointers");
        // SAFETY: Stack construction guarantees alignment and room for the
        // basic frame. The exclusive loan has never been used for execution.
        unsafe {
            stack
                .pointer(sp)
                .cast::<context::InitialFrame>()
                .write(frame)
        };
        Self {
            stack,
            sp,
            _entry: entry,
            _argument: argument,
        }
    }

    fn valid(&self, sp: usize) -> bool {
        if !self.stack.fits(sp, context::FRAME_BYTES) || !self.stack.canary_intact() {
            return false;
        }
        // SAFETY: The checked frame is either built by new or stacked by
        // hardware on exception entry. Only the hardware words are read.
        let (pc, xpsr) = unsafe {
            (
                self.stack.pointer(sp + context::PC_OFFSET).read_volatile(),
                self.stack
                    .pointer(sp + context::XPSR_OFFSET)
                    .read_volatile(),
            )
        };
        unsafe extern "C" {
            static __text_start: u8;
            static __text_end: u8;
        }
        let text = (&raw const __text_start).addr()..(&raw const __text_end).addr();
        let padding = usize::from(xpsr & context::ALIGN_PADDING != 0) * 4;
        text.contains(&(pc as usize))
            && context::valid_thread_frame(pc, xpsr)
            && self.stack.fits(sp, context::FRAME_BYTES + padding)
    }
}

#[derive(PartialEq, Eq)]
enum Boot {
    Fresh,
    Armed,
    Running,
}

struct Runtime {
    tasks: Table<Context, CAPACITY>,
    idle: Option<Context>,
    boot: Boot,
    exiting: bool,
}

struct CoreLocal(UnsafeCell<Runtime>);

// SAFETY: Access is single-core, with configurable interrupts masked.
// NMI and HardFault are terminal and never access the task table.
unsafe impl Sync for CoreLocal {}

static RUNTIME: CoreLocal = CoreLocal(UnsafeCell::new(Runtime {
    tasks: Table::new(),
    idle: None,
    boot: Boot::Fresh,
    exiting: false,
}));

pub static SWITCHES: AtomicU32 = AtomicU32::new(0);

fn with<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    interrupt::critical_section(|| {
        // SAFETY: Call sites are non-reentrant and do not yield, call thread
        // APIs, or export runtime references while this borrow is active.
        f(unsafe { &mut *RUNTIME.0.get() })
    })
}

fn check_thread() -> Result<(), Error> {
    if interrupt::in_interrupt_context() {
        return Err(Error::Caller);
    }
    Ok(())
}

fn check_unmasked() -> Result<(), Error> {
    check_thread()?;
    if !interrupt::interrupts_enabled() || interrupt::basepri() != 0 || !context::faultmask_clear()
    {
        return Err(Error::Caller);
    }
    Ok(())
}

/// Create a ready task in Thread Mode. Returns the exclusive storage on error.
pub fn create_static(
    name: &'static str,
    storage: &'static mut [u32],
    entry: context::Entry,
    argument: u32,
) -> Result<TaskId, (Error, &'static mut [u32])> {
    if let Err(error) = check_thread() {
        return Err((error, storage));
    }
    let stack = Stack::new(storage).map_err(|storage| (Error::InvalidStack, storage))?;
    let candidate = Context::new(stack, entry, argument);
    with(|rt| rt.tasks.create(name, candidate)).map_err(|(error, candidate)| {
        // SAFETY: The unsuccessful transaction did not publish the context.
        (Error::Table(error), unsafe {
            candidate.stack.into_storage()
        })
    })
}

/// Get the running task ID in Thread Mode. Boot and idle have no ID.
pub fn current() -> Result<Option<TaskId>, Error> {
    check_thread()?;
    Ok(with(|rt| rt.tasks.current()))
}

/// Inspect the lifecycle state in Thread Mode.
pub fn state(id: TaskId) -> Result<State, Error> {
    check_thread()?;
    with(|rt| rt.tasks.get(id).map(|t| t.state()).map_err(Error::Table))
}

/// Inspect a non-running task's stack in Thread Mode.
pub fn usage(id: TaskId) -> Result<anova_kernel::memory::stack::StackStats, Error> {
    check_thread()?;
    with(|rt| {
        let task = rt.tasks.get(id).map_err(Error::Table)?;
        if task.state() == State::Running {
            return Err(Error::Table(TableError::InvalidState));
        }
        // SAFETY: The borrower is running on a different stack; PendSV is
        // masked during inspection and no ISR can inspect or reuse this stack.
        Ok(unsafe { task.context.stack.usage(task.context.sp) })
    })
}

/// Reclaim a terminated task's exclusive storage from another Thread context.
pub fn reap(id: TaskId) -> Result<&'static mut [u32], Error> {
    check_thread()?;
    let task = with(|rt| rt.tasks.reap(id)).map_err(Error::Table)?;
    // SAFETY: A Thread caller only runs after the handler committed the new
    // PSP; the terminated task cannot resume and its ID was invalidated.
    Ok(unsafe { task.stack.into_storage() })
}

/// Start from the boot MSP. Failures return without arming SVC.
pub fn start(
    storage: &'static mut [u32],
    idle: context::Entry,
) -> Result<(), (Error, &'static mut [u32])> {
    let check = check_unmasked().and_then(|()| {
        if context::uses_process_stack() || with(|rt| rt.boot != Boot::Fresh) {
            return Err(Error::Started);
        }
        if with(|rt| rt.tasks.next_ready().is_none()) {
            return Err(Error::NotStarted);
        }
        Ok(())
    });
    if let Err(error) = check {
        return Err((error, storage));
    }
    let stack = Stack::new(storage).map_err(|storage| (Error::InvalidStack, storage))?;
    with(|rt| {
        rt.idle = Some(Context::new(stack, idle, 0));
        rt.boot = Boot::Armed;
    });
    // SAFETY: One-shot armed bootstrap, verified masks and Thread/MSP.
    // Startup callers have moved every resource needed by tasks to statics.
    unsafe { context::start() }
}

/// Yield in unmasked task Thread Mode. No alternate task means no switch.
pub fn yield_now() -> Result<(), Error> {
    check_unmasked()?;
    if current()?.is_none() {
        return Err(Error::NotStarted);
    }
    nvic::pend_pendsv();
    Ok(())
}

extern "C" fn exit() -> ! {
    assert_eq!(check_unmasked(), Ok(()));
    with(|rt| {
        assert!(rt.tasks.current().is_some() && !rt.exiting);
        rt.exiting = true;
    });
    nvic::pend_pendsv();
    // Terminal fallback. Never continue into an abandoned task frame.
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
extern "C" fn m4_first() -> usize {
    with(|rt| {
        assert!(rt.boot == Boot::Armed);
        let id = rt.tasks.next_ready().expect("armed first task");
        let first = &rt.tasks.get(id).unwrap().context;
        assert!(first.valid(first.sp), "M4 first context invalid");
        let sp = first.sp;
        rt.tasks.handoff(Some(id), false).unwrap();
        rt.boot = Boot::Running;
        sp
    })
}

#[unsafe(no_mangle)]
extern "C" fn m4_select(psp: usize) -> usize {
    with(|rt| {
        if rt.boot != Boot::Running {
            return 0;
        }
        let Some(id) = rt.tasks.current() else {
            return 0;
        };
        let outgoing = rt.tasks.context_mut(id).unwrap();
        let saved = psp
            .checked_sub(context::SOFTWARE_BYTES)
            .expect("M4 PSP underflow");
        assert!(outgoing.valid(saved), "M4 outgoing context invalid");
        let next = rt.tasks.next_ready();
        if next.is_none() && !rt.exiting {
            return 0;
        }
        rt.tasks.context_mut(id).unwrap().sp = saved;
        let target = match next {
            Some(id) => &rt.tasks.get(id).unwrap().context,
            None => rt.idle.as_ref().expect("idle context"),
        };
        assert!(target.valid(target.sp), "M4 incoming context invalid");
        let sp = target.sp;
        rt.tasks.handoff(next, rt.exiting).unwrap();
        rt.exiting = false;
        SWITCHES.fetch_add(1, Relaxed);
        sp
    })
}
