# Development progress

Initialized on 2026-11-27 (UTC+8). [progress.json](progress.json) is the machine-readable source of truth.

The maintainer confirmed that the repository M0 firmware passed hardware acceptance on the Elite ME V6. UART output remains unconfirmed.

States: planned, in_progress, blocked, implemented (awaiting acceptance), done. A done milestone requires all dependencies to be done and passing evidence with commit, environment, result and artifact. A blocked milestone requires a blocker field.

| ID | Milestone | Status |
|---|---|---|
| M0 | Bare-metal bootstrap | done |
| M1 | Clock and logging | done |
| M2 | Interrupts and exceptions | done |
| M3 | Memory management | done |
| M4 | Task context | done |
| M5 | Real-time scheduling | planned |
| M6 | Synchronization | planned |
| M7 | IPC and timers | planned |
| M8 | Device model | planned |
| M9 | Basic peripherals | planned |
| M10 | Buses and DMA | planned |
| M11 | LCD and touch | planned |
| M12 | Storage devices | planned |
| M13 | Communications | planned |
| M14 | File systems | planned |
| M15 | Shell | planned |
| M16 | Resource monitoring | planned |
| M17 | Graphics rendering | planned |
| M18 | GUI and applications | planned |
| M19 | Rust SDK | planned |
| M20 | C/C++ SDK | planned |
| M21 | Loadable applications | planned |
| M22 | Platform portability | planned |

M3.1–M3.6 passed [local validation](validation/M3-local-2026-11-28.md). The first confirmed-HSE image passed SWD Verify and reached M2 startup output. The maintainer subsequently reported normal running after the startup fix. The maintainer also provided HSI DWT measurements and successful M3 boot checks. The maintainer confirmed all remaining hardware checks passed. See the [M3 hardware acceptance record](validation/M3-startup-retest-2026-11-28.md#hardware-acceptance). See the [M3 hardware procedure](validation/M3-hardware-template.md).

Platform scope: all 23 tracked milestones concern the active STM32F103ZET6 track; M22 is a portability design review, not a port. Raspberry Pi 4B and other boards are deferred outside the active milestone count; no implementation, builds, emulator or hardware tests are planned for them in the current phase. See [platform matrix](boards/README.md).

M0 evidence is recorded in the [local build and ELF validation](validation/M0-local-2026-11-28.md) and the [maintainer-reported hardware acceptance](validation/M0-2026-11-28.md).

M1 passed local verification and maintainer-reported hardware acceptance. [Local validation](validation/M1-local-2026-11-28.md) · [Hardware acceptance](validation/M1-2026-11-28.md)

M2 passed [local validation](validation/M2-local-2026-11-28.md) and [maintainer-reported hardware acceptance](validation/M2-2026-11-28.md).

M4 substage evidence is in the [local validation record](validation/M4-local-2026-11-28.md).

The [M4 board record](validation/M4-hardware-2026-11-28.md) includes stress, DWT, SWD reset and maintainer-confirmed standalone checks.
