# Development roadmap

An independent Cortex-M real-time kernel. M0–M7: minimum RTOS; M8–M13: devices; M14–M18: services; M19–M22: SDK and portability.

| ID | Milestone | Dependencies | Implementation and acceptance |
|---|---|---|---|
| M0 | Bare-metal bootstrap | None | Workspace, linker script, vector table, reset handler and LED; demonstrate repository firmware blinking on hardware. |
| M1 | Clock and logging | M0 | HSI/HSE/PLL, UART and panic logging; verify clock frequency, baud rate and fallback. |
| M2 | Interrupts and exceptions | M1 | NVIC, SysTick and exception context; verify priorities and fault reporting. |
| M3 | Memory management | M2 | Memory pools, optional heap and stack watermark; verify alignment, exhaustion and reclamation. |
| M4 | Task context | M3 | TCB, MSP/PSP and PendSV; verify register preservation and stack integrity. |
| M5 | Real-time scheduling | M4 | Fixed-priority preemption, equal-priority round robin, blocking and waking; measure latency. |
| M6 | Synchronization | M5 | Mutexes, semaphores and events; test contention, timeouts and priority inversion. |
| M7 | IPC and timers | M6 | Bounded queues and software timers; test full queues, cancellation and tick wraparound. |
| M8 | Device model | M7 | Registration, capability interfaces and lifecycle; test resource conflicts. |
| M9 | Basic peripherals | M8 | GPIO, buttons, EXTI, ADC, PWM and buzzer; verify on hardware. |
| M10 | Buses and DMA | M9 | SPI, I2C and DMA; test timeouts, errors and buffer lifetimes. |
| M11 | LCD and touch | M10 | Identify controllers, implement partial drawing and calibration; verify solid colors, text and coordinates. |
| M12 | Storage devices | M11 | SPI NOR, EEPROM and SD; verify read/write integrity, capacity and power-loss risks. |
| M13 | Communications | M12 | USB Device, CAN and RS485; verify enumeration where applicable, transmission, reception and recovery. |
| M14 | File systems | M13 | VFS, FAT32 and optional LittleFS; test mounting, reads/writes, full and damaged media. |
| M15 | Shell | M14 | Command registration and UART interaction; test input boundaries and error handling. |
| M16 | Resource monitoring | M15 | Tasks, stack watermarks, CPU and RAM statistics; compare with measurements. |
| M17 | Graphics rendering | M16 | RGB565, fonts, clipping and partial refresh; measure RAM usage and latency. |
| M18 | GUI and applications | M17 | Event loop, widgets and static application management; verify touch and scheduling coexistence. |
| M19 | Rust SDK | M18 | Public APIs, examples and version policy; verify standalone application builds. |
| M20 | C/C++ SDK | M19 | C ABI, headers and toolchain; verify cross-language calls. |
| M21 | Loadable applications | M20 | Assess relocation, ABI, memory and security boundaries; implement and validate only if feasible. |
| M22 | Platform portability | M21 | Port to a second board; demonstrate shared kernel independence from STM32 registers. |

Dependencies define a conservative initial acceptance order; revise through architectural decisions. Independent driver experiments may proceed in parallel. M21 is a feasibility gate, not a promise of MCU process isolation.
