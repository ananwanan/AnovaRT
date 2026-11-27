# AnovaRT

**[English](README.md) | 简体中文**

独立于 AnovaOS、以 Rust 为主要开发语言的开源嵌入式实时操作系统（RTOS）。

## 项目目标
- 自研模块化抢占式实时内核和寄存器级驱动。
- 可移植的架构层、芯片层与板级支持层。
- 主平台及当前唯一开发目标：STM32F103ZET6（Cortex-M3，512 KiB Flash，64 KiB SRAM）。
- 未来可选平台：树莓派 4B（AArch64）及其他开发板，仅规划，尚未实现或验证。
- 可选存储、Shell、图形系统及应用 SDK。

## 平台支持策略
当前仅对手头的 STM32 开发板进行开发、编译、烧录、调试与验收。保持通用接口可扩展，但未经维护者明确启用移植任务，不对次选平台进行实现、编译、模拟器或实机测试。次选平台不作为里程碑、CI 或发布的阻塞条件。

## 当前状态
M0 裸机固件已可为 Elite ME V6 编译，维护者确认[真机验收](docs/validation/M0-2026-11-28.md)通过。编译与烧录命令见[开发指南](docs/DEVELOPMENT.md)。

## 项目文档
以下开发文档统一使用英语：
- [开发路线图](docs/ROADMAP.md)
- [进度追踪](docs/PROGRESS.md)
- [系统架构](docs/ARCHITECTURE.md)
- [开发指南](docs/DEVELOPMENT.md)
- [测试与验收](docs/TESTING.md)
- [平台支持策略与矩阵](docs/boards/README.md)
- [STM32F103ZE 开发板](docs/boards/stm32f103ze.md)
- [树莓派 4B（延期）](docs/boards/raspberry-pi-4b.md)
- [AI 开发规范](AGENTS.md)
- [贡献指南](CONTRIBUTING.md)
- [安全政策](SECURITY.md)

运行 `python scripts/check_progress.py` 校验里程碑记录。

## 分支
dev 为开发集成分支；main 保留稳定发布版本；功能分支从 dev 创建。

## 授权
采用 [Apache License 2.0](LICENSE)。在遵守协议的前提下，个人与商业使用均无需支付许可费用。保留适用的版权、许可证及 NOTICE 信息。第三方材料保留各自许可证。
