# System architecture

Applications/SDK → Shell/VFS/GUI → Kernel (task/scheduler/sync/ipc/timer/memory) → Arch/HAL/Drivers → BSP/Hardware.

M0 workspace and future source locations:
```
arch/cortex-m/       # active: reset, exceptions, context
# arch/aarch64/      # deferred candidate, not implemented
kernel/              # portable real-time kernel
hal/stm32f1/         # active chip register layer
# hal/bcm2711/       # deferred candidate
bsp/threebluewhale-f103ze/ # active board bindings
# bsp/raspberry-pi-4b/    # deferred candidate
firmware/            # active board firmware entry
drivers/
components/          # shell, vfs, gui
sdk/ apps/ tests/ xtask/
```
M0 uses the Flash boot alias, a single core, static linking and a shared address space. Hardware loads the 8-byte-aligned MSP from the vector table. Reset masks configurable interrupts, copies .data, clears .bss and calls the Rust entry. The entry checks one word from each section before configuring DS0. Thread mode and exception handlers use MSP. Cortex-M3 hardware stacks R0–R3, R12, LR, PC and xPSR on exception entry and places EXC_RETURN in LR. M0 default handlers are terminal. PSP, task switching and exception return handling are future work.

The architecture layer owns core vectors and reset assembly. The chip layer supplies external vectors and GPIO/RCC register access. The BSP supplies the linker memory map and LED binding. The firmware owns the blink loop, which uses an uncalibrated busy wait at the reset clock. The portable kernel crate has no board dependencies.

STM32 resource targets (not measurements): 512 KiB Flash and 64 KiB SRAM. The [M3 SRAM budget](memory/OVERVIEW.md#sram-budget) includes dedicated 4 KiB main and emergency stacks. Minimum kernel target: Flash <32 KiB, static RAM <4 KiB excluding application and task stacks. Use partial LCD rendering rather than a full framebuffer. Exclude file systems and GUI from M0–M7.

## M1 startup and logging

- Firmware reuses the reset entry and memory self-check, initializes DS0, then configures clocks and the optional console before printing the boot banner. Configurable interrupts remain masked. DS0 delays are sliced so the main loop can poll for single-byte USART1 echo.
- The STM32 HAL owns RCC, Flash ACR and USART1 registers. Flash latency and AHB/APB prescalers precede the PLL switch. The HAL derives the bus frequencies from the selected source and prescaler readback after switch confirmation. Recovery confirms SWS=HSI before disabling PLL/HSE or reducing latency. A failed HSI recovery parks the CPU.
- The BSP chooses whether the operator has confirmed an 8 MHz crystal and USART1 routing. It binds USART1 to PA9/PA10 and passes the current PCLK2 through `Clocks`. Clock configuration is fixed after console initialization. Clock fault injection exercises boot recovery. Runtime oscillator supervision is outside M1.
- The kernel owns the `Sink` interface, five level macros and compile-time filtering. A sink receives a level and `fmt::Arguments` for a complete record, which gives future implementations a place to add timestamps and serialization. M1 uses a mutable `fmt::Write` adapter with synchronous output.
- Firmware owns the panic handler. Atomic flags publish console readiness and guard recursive entry. The terminal path can reuse the initialized UART without resuming the interrupted writer. It caps output at 512 bytes with bounded TXE and TC polling, then parks the core. Formatting runs synchronously. A custom formatter used in a panic must itself return.

## M2 interrupts and timebase

- Reset masks configurable exceptions, initializes RAM, and writes the Flash vector base to VTOR. The table is 512-byte-aligned and spans 76 words at `0x08000000`. Reserved core slots contain zero. The HAL supplies external IRQ slots 0 through 59. M2 unmasks after configuring handlers and priorities. M0 and M1 retain their terminal default vectors and keep configurable interrupts masked.
- The architecture layer implements NVIC word banks for write-one enable, disable and pending commands, and byte accesses for priorities. The STM32F103 HAL bounds IRQ numbers to 0 through 59. AIRCR PRIGROUP=3 makes all four implemented bits preemption bits. Logical priorities are 0 (highest) through 15 (lowest), encoded in bits 7:4. NMI and HardFault have fixed architectural priorities. SHPR configures system exceptions separately. M2 assigns SVC 8, SysTick 14 and PendSV 15. Validation profiles software-pend DMA1 channel IRQs without enabling DMA.
- PRIMASK critical sections restore the exact incoming mask. NMI and HardFault remain eligible. BASEPRI ceilings use BASEPRI_MAX to preserve a stronger nested threshold. Zero requests no additional mask. Both guards restore state on normal return and use asm memory clobbers plus hardware barriers to order accesses. Target panics abort via a terminal path without running guard destructors. These interfaces require privileged single-core execution.
- SysTick uses HCLK and an integral period of 2 to 2^24 cycles. At 1 kHz, LOAD=71999 at 72 MHz and LOAD=7999 at nominal 8 MHz HSI. Start clears the current value and pending bit. Stop disables the timer and clears pending. Reading CTRL to query enable consumes COUNTFLAG. An atomic 32-bit count tracks delivered ticks with relaxed ordering. Uptime wraps after 2^32 ticks (about 49.7 days). Wrapping subtraction is valid for intervals shorter than a full wrap. Long masking, higher-priority work or debug halts can coalesce expirations into one pending exception. DS0 toggles every 500 delivered ticks, and USART1 echo stays in Thread Mode.
- Thread Mode uses MSP. Hardware pushes a 32-byte basic frame containing R0, R1, R2, R3, R12, LR, PC and xPSR in that order. CCR.STKALIGN aligns the frame to eight bytes. Stacked xPSR bit 9 signals an alignment word after the frame. Hardware places EXC_RETURN in LR. The SVC, PendSV, SysTick and test IRQ veneers tail-branch to preserve EXC_RETURN under the C ABI.
- A terminal vector masks configurable exceptions, saves IPSR and EXC_RETURN, and samples MSP/PSP before switching to the dedicated 4 KiB emergency MSP. EXC_RETURN FFFFFFF1 or FFFFFFF9 selects MSP, and FFFFFFFD selects PSP. Frame validation rejects other encodings, stacking or unstacking errors, unaligned or out-of-bounds addresses, overlap with the emergency stack and truncated padding. A valid range cannot establish trust in the context. Before VTOR is published, or if a fault or NMI reenters on the emergency stack, the bridge parks without calling Rust.
- `M2_FAULT_RECORD` captures SCB status and any validated basic frame for SWD inspection. MMFAR and BFAR have meaning only with their CFSR valid bits. CCR.DIV_0_TRP enables integer divide-by-zero trapping, and CCR.UNALIGN_TRP traps accesses that would otherwise permit misalignment. Firmware shares the M1 terminal writer with a recursion guard, bounded USART polling and a 512-byte output cap. The core parks without returning or resetting. An exception-entry failure, lockup, corrupt SRAM or NMI arriving mid-diagnostic can prevent capture. An imprecise BusFault can have an unrelated stacked PC.
- M3 owns memory pools and stack measurement. M4 owns task stacks, PSP and context switches. M5 owns scheduling policy and deferred switches through PendSV. M2 provides the tick and basic exception controls.

References are [ST PM0056](https://www.st.com/resource/en/programming_manual/cd00228163.pdf) and [ST RM0008](https://www.st.com/resource/en/reference_manual/rm0008-stm32f103xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf).

## M3 memory management

- The portable kernel owns fixed-size pools, TLSF metadata and task-stack diagnostics. The board owns its SRAM mapping and exposes linked ranges. Firmware binds pool and optional heap storage once at startup.
- The Cortex-M3 reset code paints the MSP before the first frame. The live scan samples MSP inside a stackless read loop, with configurable exceptions masked. Terminal fault handling retains access to the separate M2 emergency stack.
- The pool serves fixed blocks by address and allocation bitmap. TLSF uses two levels of size classes, bitmaps and per-class doubly linked free lists. Physical boundary tags allow split and neighbor reclamation with bounded metadata work.
- Firmware registers the heap only with `heap-tlsf`. The adapter reuses the M2 PRIMASK guard in Thread Mode. The optional heap and statistics have no impact on scheduling policy.

See the [memory overview](memory/OVERVIEW.md) for initialization, accounting and test scope.

## Portability boundaries

STM32F103ZET6 remains the only active implementation target. Raspberry Pi 4B and other boards are deferred, not supported firmware targets. The commented paths above are illustrative future locations, not a request to create them now.

| Layer | Responsibility |
|---|---|
| Kernel | Task lifecycle, scheduler policy, bounded IPC, synchronization and timer logic; no direct board MMIO or CPU-specific exception frames. |
| Architecture | Boot entry, context layout/switching, exception handling, interrupt masking and barriers; memory-management and atomic capabilities where relevant. |
| Chip HAL | Interrupt controller, timer, clock, MMIO, DMA and peripheral mechanisms specific to the SoC/MCU. |
| BSP | Board identity, memory map/linker settings, pin mapping, device instances, boot configuration and board-specific initialization. |
| Drivers/components | Reusable protocols and capabilities with target-specific hardware implementations; optional services selected according to board resources. |

Keep the initial single-core, no-MMU design. Do not assume every target is 32-bit, has an NVIC/SysTick/PendSV, shares the STM32 memory map, or needs the same interrupt/context representation. Optional MMU, SMP and cache/DMA handling on future targets are not prerequisites for STM32 work. Choose narrow interfaces as actual needs arise; do not build unused portability infrastructure. Future ports should use explicit target/board selection without changing the STM32 default. See [platform policy](boards/README.md) and the [deferred Raspberry Pi 4B plan](boards/raspberry-pi-4b.md).
