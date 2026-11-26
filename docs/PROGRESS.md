# 开发进度

初始化：2026-11-27（UTC+8）。机器可读状态以 [progress.json](progress.json) 为准。

基线：维护者已报告独立 LED Demo 可运行，SWD 已识别 Cortex-M3；串口输出未明确确认。仓库固件尚未实现，M0 不算完成。

状态：planned 待开发；in_progress 开发中；blocked 阻塞；implemented 已实现待验收；done 验收通过。done 需要依赖全部 done，并有提交、环境、结果和证据路径或 URL；blocked 需要 blocker 字段。

| ID | 内容 | 状态 |
|---|---|---|
| M0 | 裸机启动 | planned |
| M1 | 时钟与日志 | planned |
| M2 | 中断与异常 | planned |
| M3 | 内存管理 | planned |
| M4 | 任务上下文 | planned |
| M5 | 实时调度 | planned |
| M6 | 同步原语 | planned |
| M7 | IPC 与定时器 | planned |
| M8 | 设备模型 | planned |
| M9 | 基础外设 | planned |
| M10 | 总线与 DMA | planned |
| M11 | LCD 与触摸 | planned |
| M12 | 存储设备 | planned |
| M13 | 通信外设 | planned |
| M14 | 文件系统 | planned |
| M15 | Shell | planned |
| M16 | 资源监控 | planned |
| M17 | 图形渲染 | planned |
| M18 | GUI 与应用 | planned |
| M19 | Rust SDK | planned |
| M20 | C/C++ SDK | planned |
| M21 | 可加载应用 | planned |
| M22 | 跨平台 | planned |

下一步：M0 创建 Cargo workspace、启动与链接脚本、LED 板级驱动；构建并实机验收。
