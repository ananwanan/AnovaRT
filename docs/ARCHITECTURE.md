# System architecture

Applications/SDK → Shell/VFS/GUI → Kernel (task/scheduler/sync/ipc/timer/memory) → Arch/HAL/Drivers → BSP/Hardware.

Planned layout (not implemented):
```
arch/cortex-m/       # reset, exceptions, context
kernel/              # portable real-time kernel
hal/stm32f1/         # chip register layer
bsp/threebluewhale-f103ze/ # board bindings
drivers/
components/          # shell, vfs, gui
sdk/ apps/ tests/ xtask/
```
Start with a single core, static linking and a shared address space; no process-level isolation. MSP serves handlers, PSP serves tasks, PendSV switches context, SysTick provides ticks, and SVC provides controlled entry. Implement fixed-priority preemption before considering tickless operation. The architecture layer owns exception frames; the portable kernel must not access specific GPIO addresses.

Resource targets (not measurements): 512 KiB Flash and 64 KiB SRAM. Initial SRAM budget in KiB: static data 8, handler stack 4, task stacks 16, heap 12, graphics buffers 8, device buffers 4, reserve 12. Minimum kernel target: Flash <32 KiB, static RAM <4 KiB excluding application and task stacks. Use partial LCD rendering rather than a full framebuffer. Exclude file systems and GUI from M0–M7.
