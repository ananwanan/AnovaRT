# 系统架构

应用/SDK → Shell/VFS/GUI → Kernel(task/scheduler/sync/ipc/timer/memory) → Arch/HAL/Drivers → BSP/Hardware。

规划目录（尚未实现）：
```
arch/cortex-m/       # reset, exception, context
kernel/              # 通用实时内核
hal/stm32f1/          # 芯片寄存器层
bsp/threebluewhale-f103ze/ # 板级绑定
drivers/
components/          # shell, vfs, gui
sdk/ apps/ tests/ xtask/
```
初期单核、静态链接、共享地址空间，不提供进程级隔离。MSP 用于 handler，PSP 用于任务，PendSV 切换，SysTick 提供 tick，SVC 用于受控入口。固定优先级抢占先于 tickless。架构层维护异常帧；内核不读取具体 GPIO 地址。

资源目标（非实测）：Flash 512 KiB、SRAM 64 KiB；静态数据 8、handler 栈 4、任务栈 16、堆 12、图形缓冲 8、设备缓冲 4、余量 12 KiB。最小内核目标 Flash <32 KiB、静态 RAM <4 KiB（不含应用及任务栈）。LCD 采用分块/局部刷新，无全屏帧缓冲。M0–M7 不引入文件系统和 GUI。
