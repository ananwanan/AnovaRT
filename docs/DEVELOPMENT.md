# Development guide

The active STM32 workflow uses Windows PowerShell or Linux, VS Code, Rust stable and probe-rs. Pin rust-toolchain.toml in M0 and record versions.

```powershell
git clone --branch dev https://github.com/ananwanan/AnovaRT.git
cd AnovaRT
rustup target add thumbv7m-none-eabi
cargo install probe-rs-tools --locked
probe-rs list
probe-rs info --chip STM32F103ZETx
python scripts/check_progress.py
```
Documentation only at present. After M0 establishes a workspace and runner, use cargo fmt --all -- --check, cargo build --release --target thumbv7m-none-eabi, and cargo run --release --target thumbv7m-none-eabi. Running overwrites Flash; confirm first. Use explicit host targets or separate crates for host tests. Windows ST-Link driver: https://www.st.com/en/development-tools/stsw-link009.html. JTAG failure with successful SWD is not a blocker.

Branch policy: dev is the intended default and integration branch; main is for stable releases; create feature branches such as feat/m0-bootstrap from dev. Each milestone PR must link acceptance records. Never force-push shared branches.

## Target selection and deferred boards

The only active target is thumbv7m-none-eabi on the current STM32F103ZET6 board. M0 should establish explicit board selection and a board-specific linker/runner configuration without making other targets mandatory. Host-only tests of portable logic remain applicable. Raspberry Pi 4B (candidate aarch64-unknown-none) and other boards are documentation-only deferred targets. Do not install their toolchains, add Cargo targets or runners, generate firmware images, run emulators, flash or debug them for current milestones. The Raspberry Pi target is a future bare-metal port, not a Linux application. See [platform policy](boards/README.md).
