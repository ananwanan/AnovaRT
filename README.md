# AnovaRT

**English | [简体中文](README.zh-CN.md)**

A Rust-first, open-source embedded real-time operating system (RTOS), independent of AnovaOS.

## Goals
- Self-developed, modular preemptive real-time kernel and register-level drivers.
- Portable architecture, chip and board support layers.
- First target: STM32F103ZET6 (Cortex-M3, 512 KiB Flash, 64 KiB SRAM).
- Optional storage, shell, graphics and application SDKs.

## Status
The maintainer has confirmed that a standalone bare-metal LED demo runs on hardware. It has not been integrated into this repository. No RTOS firmware is available for release yet.

## Documentation
- [Roadmap](docs/ROADMAP.md)
- [Progress](docs/PROGRESS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Testing and acceptance](docs/TESTING.md)
- [Board support](docs/boards/stm32f103ze.md)
- [Agent instructions](AGENTS.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

Run `python scripts/check_progress.py` to validate milestone records.

## Branches
Development takes place on dev; main is reserved for stable releases. Feature branches start from dev.

## License
[Apache License 2.0](LICENSE). Personal and commercial use are royalty-free under its terms. Retain applicable copyright, license and NOTICE information. Third-party materials retain their respective licenses.
