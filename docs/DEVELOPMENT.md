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

## Target selection and deferred boards

The only active target is thumbv7m-none-eabi on the current STM32F103ZET6 board. The M0 Cargo aliases select the firmware package, target and board-specific linker and runner. Host-only tests of portable logic remain applicable. Raspberry Pi 4B (candidate aarch64-unknown-none) and other boards are documentation-only deferred targets. Do not install their toolchains, add Cargo targets or runners, generate firmware images, run emulators, flash or debug them for current milestones. The Raspberry Pi target is a future bare-metal port, not a Linux application. See [platform policy](boards/README.md).
