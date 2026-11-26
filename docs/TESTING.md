# 测试与验收

分层：宿主纯逻辑单测；thumbv7m-none-eabi 交叉构建；实机 SWD/串口/GPIO；必要时逻辑分析仪测时序。QEMU 可辅助但不可替代板级验证。

记录提交 SHA、日期、工具版本、板卡、命令、退出码、预期和实际结果、证据路径或 URL、未执行项。记录到 docs/validation/Mn-YYYY-MM-DD.md；progress.json 的 evidence 项含 commit、environment、result（pass）、artifact。所有验收点均通过才标 done；无设备时标 implemented 或 blocked，不伪造。

调度验收包含寄存器保留、栈边界、抢占顺序、并发、超时、tick 回绕、ISR 延迟和上下文切换测量。报告测量条件及最大值，不用平均值声称硬实时。内存验收检查链接 map、栈水位、OOM、越界及碎片。驱动包含错误路径、超时与重试上限；存储测试使用已备份的测试介质。
