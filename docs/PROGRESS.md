# Development progress

Initialized on 2026-11-27 (UTC+8). [progress.json](progress.json) is the machine-readable source of truth.

The maintainer confirmed that the repository M0 firmware passed hardware acceptance on the Elite ME V6. UART output remains unconfirmed.

States: planned, in_progress, blocked, implemented (awaiting acceptance), done. A done milestone requires all dependencies to be done and passing evidence with commit, environment, result and artifact. A blocked milestone requires a blocker field.

| ID | Milestone | Status |
|---|---|---|
| M0 | Bare-metal bootstrap | done |
| M1 | Clock and logging | in_progress |
| M2 | Interrupts and exceptions | planned |
| M3 | Memory management | planned |
| M4 | Task context | planned |
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

Platform scope: all 23 tracked milestones concern the active STM32F103ZET6 track; M22 is a portability design review, not a port. Raspberry Pi 4B and other boards are deferred outside the active milestone count; no implementation, builds, emulator or hardware tests are planned for them in the current phase. See [platform matrix](boards/README.md).

M0 evidence is recorded in the [local build and ELF validation](validation/M0-local-2026-11-28.md) and the [maintainer-reported hardware acceptance](validation/M0-2026-11-28.md).
