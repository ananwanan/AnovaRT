# 开发指南

Windows PowerShell + VS Code + Rust stable + probe-rs；Linux 可选。M0 固定 rust-toolchain.toml 并记录版本。
```powershell
git clone https://github.com/ananwanan/AnovaRT.git
cd AnovaRT
rustup target add thumbv7m-none-eabi
cargo install probe-rs-tools --locked
probe-rs list
probe-rs info --chip STM32F103ZETx
python scripts/check_progress.py
```
目前只有文档。M0 建立 workspace/runner 后再执行 cargo fmt --all -- --check、cargo build --release --target thumbv7m-none-eabi、cargo run --release --target thumbv7m-none-eabi。run 覆盖 Flash。宿主单测使用明确 host target 或独立 crate。Windows ST-Link 驱动：https://www.st.com/en/development-tools/stsw-link009.html；JTAG 失败且 SWD 成功不阻塞。

分支：main 稳定发布，dev 集成，feat/m0-bootstrap 等功能分支。每阶段 PR 关联验收记录，禁止强推共享分支。
