# AnovaRT

**English | [简体中文](README.zh-CN.md)**

A Rust-first, open-source embedded real-time operating system (RTOS), independent of AnovaOS.

## Goals
- Self-developed, modular preemptive real-time kernel and register-level drivers.
- Portable architecture, chip and board support layers.
- Primary and only active target: STM32F103ZET6 (Cortex-M3, 512 KiB Flash, 64 KiB SRAM).
- Future optional targets: Raspberry Pi 4 Model B (AArch64) and other boards. These are planned, not implemented or validated.
- Optional storage, shell, graphics and application SDKs.

## Platform policy
Develop, build, flash, debug and validate only the current STM32 board. Keep portable interfaces extensible, but do not implement, build, emulate or test secondary boards unless the maintainer explicitly activates a port. Secondary targets do not gate milestones, CI or releases.

## Status
The M0 bare-metal firmware builds for the Elite ME V6 and has passed maintainer-reported [hardware acceptance](docs/validation/M0-2026-11-28.md). See the [development guide](docs/DEVELOPMENT.md) for build and flash commands.

## Documentation
- [Roadmap](docs/ROADMAP.md)
- [Progress](docs/PROGRESS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Testing and acceptance](docs/TESTING.md)
- [Platform support policy and matrix](docs/boards/README.md)
- [STM32F103ZE board](docs/boards/stm32f103ze.md)
- [Raspberry Pi 4B (deferred)](docs/boards/raspberry-pi-4b.md)
- [Agent instructions](AGENTS.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

Run `python scripts/check_progress.py` to validate milestone records.

## Branches
Development takes place on dev; main is reserved for stable releases. Feature branches start from dev.

## License
[Apache License 2.0](LICENSE). Personal and commercial use are royalty-free under its terms. Retain applicable copyright, license and NOTICE information. Third-party materials retain their respective licenses.
