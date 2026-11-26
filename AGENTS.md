# AGENTS.md — AnovaRT

## 开工
- 检查 git status --short --branch、当前提交和工作区；保留用户修改，不擅自 reset/clean/force-push。
- 阅读 README、docs/ARCHITECTURE.md、ROADMAP.md、PROGRESS.md、progress.json、DEVELOPMENT.md、TESTING.md 与板级资料。
- 选定单个里程碑，明确依赖、验收、非目标与风险。默认 dev 集成，feat/mN-topic 开发，main 发布；未经授权不得合并、发布或更改许可证。

## 实现约束
- Rust no_std，内核与驱动自研，不使用现成 RTOS 或厂商 HAL；允许审查过的底层辅助 crate。依赖版本和许可证须记录。
- 通用内核与 arch/HAL/BSP 分离；初始 target thumbv7m-none-eabi，不假定 MMU/FPU/MPU。
- unsafe 最小化并逐处说明 SAFETY；MMIO 使用 volatile，明确宽度、对齐、保留位、时钟和并发约束；volatile 不是同步机制。
- ISR 不阻塞，不无界等待；临界区有界并恢复中断状态。任务栈 8 字节对齐，说明 MSP/PSP、异常帧、EXC_RETURN。
- 避免无界队列和隐式分配；驱动有超时与错误返回。未确认板级引脚不得盲目驱动。
- rustfmt、英文标识符；原创代码 SPDX-License-Identifier: Apache-2.0。不得复制公司内网或不明授权代码；第三方保留原许可。

## 验证和进度
- 执行适用的 fmt、host tests、target build、clippy，记录命令、退出码、提交、环境、日志、资源占用；未执行明确标记。
- 独立 Demo、仅编译或模拟器成功均不能替代仓库固件实机验收。烧录前提示覆盖 Flash，获得操作者确认。
- 维护 docs/progress.json 与 PROGRESS.md，运行 python scripts/check_progress.py；满足全部验收和依赖、具备提交与证据才允许 done；blocked 写原因。
- Conventional Commits；PR 写明变更、风险、测试、未验证项。不得虚构进度、性能、硬件或测试结果。
