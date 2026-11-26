# AnovaRT

A Rust-first, open-source embedded RTOS, independent of AnovaOS.

面向微控制器的自研 Rust 实时操作系统。首个目标：STM32F103ZET6（Cortex-M3，512 KiB Flash，64 KiB SRAM）。

## 状态
独立 LED 裸机 Demo 已由维护者确认可运行，尚未集成本仓库。当前没有可发布的 RTOS 固件。

## 文档
- [路线图](docs/ROADMAP.md)
- [进度追踪](docs/PROGRESS.md)
- [架构](docs/ARCHITECTURE.md)
- [开发指南](docs/DEVELOPMENT.md)
- [测试规范](docs/TESTING.md)
- [板级资料](docs/boards/stm32f103ze.md)
- [AI 开发规范](AGENTS.md)
- [贡献指南](CONTRIBUTING.md)
- [安全政策](SECURITY.md)

运行 `python scripts/check_progress.py` 检查进度记录。

## 授权
[Apache License 2.0](LICENSE)：个人和商业用途均免费，无需另行付费授权；使用、修改、分发应遵循许可证及 NOTICE 要求。第三方材料保留原许可。商标权不随软件授权转让。
