# 首个目标板：三只蓝鲸精英 ME V6

STM32F103ZET6，Cortex-M3，512 KiB Flash（0x08000000），64 KiB SRAM（0x20000000），最高 72 MHz。目标 thumbv7m-none-eabi。参考 ST 数据手册 https://www.st.com/resource/en/datasheet/stm32f103ze.pdf 与 RM0008。

已由维护者报告：ST-Link V2.1 识别、SWD 连通、独立 LED demo 可烧录运行。候选引脚依据此前照片和 demo：DS0 PB5、DS1 PE5、USART1 PA9/PA10；UART 输出待验收。ST-Link USB 驱动已在维护者环境解决。BOOT0 低电平从 Flash 启动；正常连接不随意更改跳线。

LCD 控制器、分辨率、触摸控制器、总线、其他外设及引脚均须核对原理图/芯片丝印/实测后写入 BSP；不得假定型号。首次烧录确认原固件是否需备份。
