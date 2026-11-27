# Platform support policy

AnovaRT is designed for multiple architectures, but the project currently develops and validates only one physical board. A planned target is not a supported firmware target.

| Board | Architecture / candidate Rust target | Priority | Implementation and validation |
|---|---|---|---|
| [Three Blue Whales Elite ME V6 / STM32F103ZET6](stm32f103ze.md) | Cortex-M3 / thumbv7m-none-eabi | Primary, sole active target | Planned repository firmware; standalone LED demo and SWD reported separately. All active hardware acceptance is on this board. |
| [Raspberry Pi 4 Model B](raspberry-pi-4b.md) | Cortex-A72 / aarch64-unknown-none | Secondary, deferred | Documentation only; not implemented, built, booted or tested. |
| Other development boards | To be selected | Secondary, deferred | No board selected; no implementation or validation. |

## Rules

- Develop, cross-compile, flash, debug and validate only the active STM32 board; retain host tests for portable logic. No secondary cross-build, emulation, hardware test, performance work, CI matrix or release gate is required or authorized during this phase.
- Keep architecture-specific mechanisms, chip HAL and BSP separate from portable kernel policies. Avoid speculative implementation and abstractions that compromise the active MCU's resources or schedule.
- Deferred platforms are excluded from active milestone counts; M22 reviews portability boundaries without requiring a port. Do not represent deferred targets as supported or tested.
- To activate a port, the maintainer must explicitly approve the exact board, scope, resource budget, toolchain, acceptance plan and priority. Create separate port milestones and validation records, and update this matrix, the roadmap and both READMEs. Keep STM32 acceptance independent unless the maintainer explicitly changes the policy.
