# Primary and only active target board: Three Blue Whales Elite ME V6

STM32F103ZET6, Cortex-M3, 512 KiB Flash (0x08000000), 64 KiB SRAM (0x20000000), up to 72 MHz. Target: thumbv7m-none-eabi. Refer to the ST datasheet https://www.st.com/resource/en/datasheet/stm32f103ze.pdf and RM0008.

The maintainer reported a working ST-Link V2.1 SWD connection and a standalone LED demo using DS0 on PB5 and DS1 on PE5 in push-pull mode. The M0 BSP drives DS0 on PB5 in 2 MHz push-pull mode with active-low polarity. It preloads the inactive latch before enabling the output. The maintainer reported that [M0 repository firmware passed hardware acceptance](../validation/M0-2026-11-28.md). USART1 PA9/PA10 remain candidate pins and UART output awaits acceptance. BOOT0 low selects Flash boot.

Verify LCD controller, resolution, touch controller, bus, other peripherals and pins against schematics, markings or measurements before implementing BSP bindings. Confirm whether original firmware requires backup before flashing.

This board is the only current development, build, flash, debug and hardware acceptance target. See [platform support policy](README.md). Deferred boards do not change the STM32 resource budget or acceptance requirements.
