# AGENTS.md — AnovaRT

## Before starting
- Inspect git status --short --branch, current commit and workspace. Preserve user changes; do not reset, clean or force-push without authorization.
- Read README.md, docs/ARCHITECTURE.md, docs/ROADMAP.md, docs/PROGRESS.md, docs/progress.json, docs/DEVELOPMENT.md, docs/TESTING.md and docs/boards/README.md plus relevant board documentation.
- Select one milestone; establish dependencies, acceptance criteria, non-goals and risks. Use dev for integration, feat/mN-topic for development and main for releases. Do not merge, release or change licensing without authorization.

## Implementation rules
- Use Rust no_std and a self-developed kernel and drivers; no existing RTOS or vendor HAL implementation. Reviewed low-level helper crates are allowed; record dependency versions and licenses.
- Separate portable kernel logic from architecture, HAL and BSP. The only active target is thumbv7m-none-eabi on the Three Blue Whales STM32F103ZET6 board. Do not assume an MMU, FPU or MPU in portable code, or assume Cortex-M exception frames, pointer width, memory maps or interrupt controllers on other architectures.
- Minimize unsafe code and document SAFETY invariants. Use volatile MMIO, respecting access widths, alignment, reserved bits, clocks and concurrency; volatile is not synchronization.
- ISRs must not block or wait indefinitely. Bound critical sections and restore interrupt state. On Cortex-M, align task stacks to 8 bytes and document MSP/PSP, exception frames and EXC_RETURN.
- Avoid unbounded queues and implicit allocations. Drivers need timeouts and error handling. Do not drive unverified board pins.
- Use rustfmt and English identifiers/comments. Original source files use SPDX-License-Identifier: Apache-2.0. Never copy proprietary internal or unknown-license code; preserve third-party terms.

## Platform scope and priorities
- STM32F103ZET6 is the sole active implementation, build, debug and hardware acceptance target. Raspberry Pi 4B and all other boards are deferred secondary targets; see docs/boards/README.md.
- Preserve practical portability through architecture, HAL and BSP boundaries as STM32 work proceeds. Do not create speculative backends, install secondary toolchains, add secondary build/CI jobs, emulate, flash, debug or test secondary boards without explicit maintainer activation.
- Secondary platform work is outside active milestones and release gates. M22 is a portability design review, not a second-board port. Do not report deferred or untested platforms as supported, implemented, passing or blocked milestones.
- Keep hardware-specific assumptions inside the relevant architecture/chip/board layer. Prioritize STM32 correctness and resource constraints over speculative abstractions.

## Language policy
- Maintain equivalent English README.md and Simplified Chinese README.zh-CN.md, with mutual links.
- All other documentation, progress data, code comments, commit messages and PR descriptions should be in English. Update both READMEs when shared content changes.

## Verification and progress
- Run applicable formatting, host tests, STM32 target builds and Clippy. Record commands, exit codes, commits, environments, logs and resource usage. Explicitly identify tests not run.
- A standalone demo, successful compilation or emulator test does not replace hardware acceptance of repository firmware. Warn before overwriting Flash and obtain operator confirmation.
- Update docs/progress.json and docs/PROGRESS.md together; run python scripts/check_progress.py. Mark done only after every acceptance criterion and dependency passes with evidence. Record blockers.
- Use Conventional Commits. PRs must describe changes, risks, tests and unverified items. Never invent progress, performance, hardware or test results.
