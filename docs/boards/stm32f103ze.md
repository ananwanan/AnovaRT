# Initial target board: Three Blue Whales Elite ME V6

STM32F103ZET6, Cortex-M3, 512 KiB Flash (0x08000000), 64 KiB SRAM (0x20000000), up to 72 MHz. Target: thumbv7m-none-eabi. Refer to the ST datasheet https://www.st.com/resource/en/datasheet/stm32f103ze.pdf and RM0008.

Maintainer-reported results: ST-Link V2.1 detected, SWD connected, standalone LED demo flashed and running. Candidate pins from earlier photos and demo: DS0 PB5, DS1 PE5, USART1 PA9/PA10; UART output awaits acceptance. ST-Link USB driver issue resolved in the maintainer environment. BOOT0 low selects Flash boot; avoid unnecessary jumper changes.

Verify LCD controller, resolution, touch controller, bus, other peripherals and pins against schematics, markings or measurements before implementing BSP bindings. Confirm whether original firmware requires backup before flashing.
