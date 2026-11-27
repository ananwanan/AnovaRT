# Raspberry Pi 4 Model B — deferred platform candidate

Status: secondary / deferred / documentation only. The candidate configuration is Raspberry Pi 4 Model B with 8 GiB RAM (the maintainer's board variant), BCM2711 and 64-bit Arm Cortex-A72. Candidate bare-metal Rust target: aarch64-unknown-none. The actual available memory map must be determined when a port is activated; the installed RAM capacity is not a usable-memory measurement. No AnovaRT firmware, board backend, image, build or test result exists for this platform.

## Proposed porting scope (future only)

| Area | Candidate approach |
|---|---|
| Boot and image | Raspberry Pi firmware boot chain with an AArch64 bare-metal image; establish entry state, linker layout and early stack. |
| Architecture | AArch64 exception vectors, context switching, interrupt masking, barriers and ABI-aligned stacks; establish exception level and memory attributes. |
| Chip HAL | BCM2711 interrupt controller, architectural timer, GPIO and UART; assess MMIO address translation and cache/DMA coherence. |
| BSP | Raspberry Pi 4B board configuration, boot media, device mapping and memory discovery. |
| Kernel | Reuse portable scheduler, IPC, synchronization and timer policies; initially use a single core. |
| Optional later work | SD storage, USB, networking, display, MMU and SMP only after a separate design decision and port activation. |

Illustrative future paths: arch/aarch64/, hal/bcm2711/, bsp/raspberry-pi-4b/. These paths do not exist as part of this documentation change. The STM32 target remains the default development target. Do not install an AArch64 toolchain, generate an image, configure CI, run emulators or perform board tests now. No secondary-platform task may delay STM32 development.

If the maintainer later activates this port, define separate boot, UART, timer, interrupt and scheduling acceptance gates with hardware evidence before reporting support. Reassess firmware requirements and hardware documentation at that time. This is a native bare-metal RTOS port, not an application hosted on Raspberry Pi OS.

See [platform policy](README.md) and [system architecture](../ARCHITECTURE.md).
