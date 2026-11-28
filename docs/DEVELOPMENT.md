# Development guide

The active STM32 workflow uses Rustup, Rust stable 1.100.0 and probe-rs 0.32.0. Rustup selects the toolchain, components and target from rust-toolchain.toml. The workspace has no external Cargo dependencies.

```powershell
git clone --branch dev https://github.com/ananwanan/AnovaRT.git
cd AnovaRT
cargo m0-build
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --workspace --release --target thumbv7m-none-eabi -- -D warnings
python scripts/check_progress.py
```
Build output is `target/thumbv7m-none-eabi/release/anova-firmware`. Install the optional flashing tool with `cargo install probe-rs-tools --version 0.32.0 --locked`. See [M0 hardware acceptance](TESTING.md#m0-hardware-acceptance) before using the `cargo m0-run` runner. Windows ST-Link drivers are available from [STMicroelectronics](https://www.st.com/en/development-tools/stsw-link009.html).

Branch policy: dev is the intended default and integration branch; main is for stable releases; create feature branches such as feat/m0-bootstrap from dev. Each milestone PR must link acceptance records. Never force-push shared branches.

## M1 build profiles

Run `cargo m1-build` for the default HSI image with candidate USART1 pins untouched. The hardware procedure shows how to select the independent confirmation features, create separate artifacts and verify programming. [M1 hardware acceptance](TESTING.md#m1-hardware-acceptance)

| Alias | Firmware |
|---|---|
| `cargo m0-build` / `cargo m0-run` | Independent M0 firmware and runner |
| `cargo m1-build` | M1 normal startup |
| `cargo m1-hse-fallback-build` | Inject HSE startup failure |
| `cargo m1-pll-fallback-build` | Inject PLL lock failure |
| `cargo m1-switch-fallback-build` | Inject SYSCLK switch failure |
| `cargo m1-panic-build` | Trigger a startup panic |

Enable only one fault or panic test profile per build. Add `--features log-debug` or `--features log-trace` to increase compile-time log verbosity. The default minimum level is INFO. Use `--target-dir` to keep simultaneous ELF artifacts distinct. Host filter tests run with `cargo test --locked -p anova-kernel --features log-debug` and again with `log-trace`.

## Target selection and deferred boards

The only active target is thumbv7m-none-eabi on the current STM32F103ZET6 board. The M0 Cargo aliases select the firmware package, target and board-specific linker and runner. Host-only tests of portable logic remain applicable. Raspberry Pi 4B (candidate aarch64-unknown-none) and other boards are documentation-only deferred targets. Do not install their toolchains, add Cargo targets or runners, generate firmware images, run emulators, flash or debug them for current milestones. The Raspberry Pi target is a future bare-metal port, not a Linux application. See [platform policy](boards/README.md).

## Build M2

```powershell
cargo m2-build
cargo m2-build --features confirmed-hse-8mhz,confirmed-usart1 --target-dir target/m2-board/normal
python scripts/check_m2_elf.py target/m2-board/normal/thumbv7m-none-eabi/release/anova-firmware
```

The default build uses HSI and leaves USART1 pins untouched. The confirmed
build uses the validated 8 MHz crystal and USART1 mapping. M2 takes precedence
over its M1 dependency and shares its clock and console setup. Select at most
one interrupt, fault, clock-injection or panic test per target build. Board
confirmation and log verbosity flags can accompany one profile. Use
`cargo m2-run` only with operator approval to overwrite Flash. The
[hardware procedure](validation/M2-hardware-template.md) covers building
separate images, capturing evidence and restoring the normal firmware.


## M3 firmware

```powershell
cargo m3-build
cargo m3-no-heap-build
python scripts/check_m3_elf.py target/m3/thumbv7m-none-eabi/release/anova-firmware --profile tlsf
python scripts/check_m3_elf.py target/m3-no-heap/thumbv7m-none-eabi/release/anova-firmware --profile no-heap
```

The default aliases use HSI and avoid USART1 pin configuration. Enable the
existing confirmation features only after confirming the board. The M3 aliases
write to their own directories under `target`. For a different clock or
console configuration, invoke `cargo build` with an isolated target directory
as shown in the [board procedure](validation/M3-hardware-template.md).

Use `ANOVART_HEAP_BYTES` for a different TLSF arena reservation. The firmware
accepts an eight-byte multiple from 4,096 through 65,536 before linking. The
linker enforces the actual available SRAM. Pass the same size to the ELF
checker with `--heap-bytes`. Clear the environment override after the custom
build. `cargo m3-run` invokes the existing SWD runner and overwrites Flash.
Follow the approval procedure before running it.

## M4 task context

Build with `cargo m4-build` for TLSF or `cargo m4-no-heap-build` for static
stacks without a heap. The corresponding run aliases overwrite Flash.
The [M4 procedure](validation/M4-hardware-template.md) lists board approvals,
isolated clock/allocator profiles, verified downloads, UART and standalone
reset checks. See the [design](task/OVERVIEW.md) and [implementation plan](task/PLAN.md).
