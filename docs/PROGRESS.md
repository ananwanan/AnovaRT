# Development progress

Initialized on 2026-11-27 (UTC+8). [progress.json](progress.json) is the machine-readable source of truth.

Baseline: the maintainer reported a working standalone LED demo and successful SWD identification of Cortex-M3. UART output remains unconfirmed. Repository firmware is not implemented, so M0 is not done.

States: planned, in_progress, blocked, implemented (awaiting acceptance), done. A done milestone requires all dependencies to be done and passing evidence with commit, environment, result and artifact. A blocked milestone requires a blocker field.

| ID | Milestone | Status |
|---|---|---|
| M0 | Bare-metal bootstrap | planned |
| M1 | Clock and logging | planned |
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

Next: M0, create Cargo workspace, startup code, linker script and board LED driver; build and validate on hardware.
