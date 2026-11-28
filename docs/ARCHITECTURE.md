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

STM32 resource targets (not measurements): 512 KiB Flash and 64 KiB SRAM. Initial SRAM budget in KiB: static data 8, handler stack 4, task stacks 16, heap 12, graphics buffers 8, device buffers 4, reserve 12. Minimum kernel target: Flash <32 KiB, static RAM <4 KiB excluding application and task stacks. Use partial LCD rendering rather than a full framebuffer. Exclude file systems and GUI from M0–M7.

## M1 startup and logging

- Firmware reuses the reset entry and memory self-check, initializes DS0, then configures clocks and the optional console before printing the boot banner. Configurable interrupts remain masked. DS0 delays are sliced so the main loop can poll for single-byte USART1 echo.
- The STM32 HAL owns RCC, Flash ACR and USART1 registers. Flash latency and AHB/APB prescalers precede the PLL switch. The HAL derives the bus frequencies from the selected source and prescaler readback after switch confirmation. Recovery confirms SWS=HSI before disabling PLL/HSE or reducing latency. A failed HSI recovery parks the CPU.
- The BSP chooses whether the operator has confirmed an 8 MHz crystal and USART1 routing. It binds USART1 to PA9/PA10 and passes the current PCLK2 through `Clocks`. Clock configuration is fixed after console initialization. Clock fault injection exercises boot recovery. Runtime oscillator supervision is outside M1.
- The kernel owns the `Sink` interface, five level macros and compile-time filtering. A sink receives a level and `fmt::Arguments` for a complete record, which gives future implementations a place to add timestamps and serialization. M1 uses a mutable `fmt::Write` adapter with synchronous output.
- Firmware owns the panic handler. Atomic flags publish console readiness and guard recursive entry. The terminal path can reuse the initialized UART without resuming the interrupted writer. It caps output at 512 bytes with bounded TXE and TC polling, then parks the core. Formatting runs synchronously. A custom formatter used in a panic must itself return.

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
