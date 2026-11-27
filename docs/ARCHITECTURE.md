# System architecture

Applications/SDK → Shell/VFS/GUI → Kernel (task/scheduler/sync/ipc/timer/memory) → Arch/HAL/Drivers → BSP/Hardware.

Planned layout (not implemented):
```
arch/cortex-m/       # active: reset, exceptions, context
# arch/aarch64/      # deferred candidate, not implemented
kernel/              # portable real-time kernel
hal/stm32f1/         # active chip register layer
# hal/bcm2711/       # deferred candidate
bsp/threebluewhale-f103ze/ # active board bindings
# bsp/raspberry-pi-4b/    # deferred candidate
drivers/
components/          # shell, vfs, gui
sdk/ apps/ tests/ xtask/
```
For the active STM32 target, start with a single core, static linking and a shared address space; no process-level isolation. MSP serves handlers, PSP serves tasks, PendSV switches context, SysTick provides ticks, and SVC provides controlled entry. Implement fixed-priority preemption before considering tickless operation. The architecture layer owns exception frames; the portable kernel must not access specific GPIO addresses.

STM32 resource targets (not measurements): 512 KiB Flash and 64 KiB SRAM. Initial SRAM budget in KiB: static data 8, handler stack 4, task stacks 16, heap 12, graphics buffers 8, device buffers 4, reserve 12. Minimum kernel target: Flash <32 KiB, static RAM <4 KiB excluding application and task stacks. Use partial LCD rendering rather than a full framebuffer. Exclude file systems and GUI from M0–M7.

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
