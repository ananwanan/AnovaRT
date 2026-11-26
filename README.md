# AnovaRT

A Rust-first, source-available embedded real-time operating system (RTOS), independent of AnovaOS.

面向微控制器的自研 Rust 实时操作系统。首个目标：三只蓝鲸精英 ME V6 / STM32F103ZET6（Cortex-M3，512 KiB Flash，64 KiB SRAM）。

## 状态

项目初始化阶段；独立 LED 裸机 Demo 已由维护者确认可运行，但尚未集成到本仓库。当前没有可发布的 RTOS 固件。

## 文档

- [开发路线图](docs/ROADMAP.md)
- [进度追踪](docs/PROGRESS.md)
- [系统架构](docs/ARCHITECTURE.md)
- [开发与构建](docs/DEVELOPMENT.md)
- [测试与验收](docs/TESTING.md)
- [板级资料](docs/boards/stm32f103ze.md)
- [AI 开发规范](AGENTS.md)
- [贡献指南](CONTRIBUTING.md)
- [安全政策](SECURITY.md)

## 授权

项目原创代码采用 PolyForm Noncommercial 1.0.0，允许其规定的非商业用途；超出公开许可范围的商业用途须另行取得书面商业授权。详见 [LICENSE](LICENSE) 和 [COMMERCIAL.md](COMMERCIAL.md)。本项目为源码公开（source-available），非 OSI 定义的开源软件。
