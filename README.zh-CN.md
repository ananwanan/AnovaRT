# AnovaRT

**[English](README.md) | 简体中文**

独立于 AnovaOS、以 Rust 为主要开发语言的开源嵌入式实时操作系统（RTOS）。

## 项目目标
- 自研模块化抢占式实时内核和寄存器级驱动。
- 可移植的架构层、芯片层与板级支持层。
- 首个目标：STM32F103ZET6（Cortex-M3，512 KiB Flash，64 KiB SRAM）。
- 可选存储、Shell、图形系统及应用 SDK。

## 当前状态
维护者已确认独立裸机 LED Demo 可在实机运行，但尚未集成到本仓库。目前没有可发布的 RTOS 固件。

## 项目文档
以下开发文档统一使用英语：
- [开发路线图](docs/ROADMAP.md)
- [进度追踪](docs/PROGRESS.md)
- [系统架构](docs/ARCHITECTURE.md)
- [开发指南](docs/DEVELOPMENT.md)
- [测试与验收](docs/TESTING.md)
- [板级支持](docs/boards/stm32f103ze.md)
- [AI 开发规范](AGENTS.md)
- [贡献指南](CONTRIBUTING.md)
- [安全政策](SECURITY.md)

运行 `python scripts/check_progress.py` 校验里程碑记录。

## 分支
dev 为开发集成分支；main 保留稳定发布版本；功能分支从 dev 创建。

## 授权
采用 [Apache License 2.0](LICENSE)。在遵守协议的前提下，个人与商业使用均无需支付许可费用。保留适用的版权、许可证及 NOTICE 信息。第三方材料保留各自许可证。
