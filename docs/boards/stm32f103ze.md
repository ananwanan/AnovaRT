# Primary and only active target board: Three Blue Whales Elite ME V6

STM32F103ZET6, Cortex-M3, 512 KiB Flash (0x08000000), 64 KiB SRAM (0x20000000), up to 72 MHz. Target: thumbv7m-none-eabi. Refer to the ST datasheet https://www.st.com/resource/en/datasheet/stm32f103ze.pdf and RM0008.

The maintainer reported a working ST-Link V2.1 SWD connection and a standalone LED demo using DS0 on PB5 and DS1 on PE5 in push-pull mode. The BSP drives DS0 on PB5 in 2 MHz push-pull mode with active-low polarity. It preloads the inactive latch before enabling the output. The maintainer reported that [M0 repository firmware passed hardware acceptance](../validation/M0-2026-11-28.md). BOOT0 low selects Flash boot.

M1 uses the following operator-confirmed configuration inputs. The maintainer reported successful hardware acceptance on the tested board. [Hardware acceptance record](../validation/M1-2026-11-28.md)

| Input | Firmware behavior |
|---|---|
| `confirmed-hse-8mhz` | Selects an 8 MHz crystal in oscillator mode, HSE ×9 PLL and 72/72/36/72 MHz SYSCLK/HCLK/PCLK1/PCLK2. Verify the crystal marking or schematics for this revision, or measure it, before enabling. With the feature omitted the clock is nominal 8 MHz HSI. |
| `confirmed-usart1` | Enables the STM32F103 default mapping (PA9 TX alternate push-pull, PA10 RX floating input), 115200 8N1 and no flow control. Confirm the routing and that PA9 can be driven. Identify the actual header, level shifter or VCP connection and isolation jumpers. With the feature omitted firmware leaves PA9/PA10 untouched. |

The MCU data sheet establishes the default pin functions. Continuity, board documentation or measurements must establish the PCB connection and signal levels. Windows enumeration of an ST-Link VCP does not establish its MCU-side routing. See [ST RM0008](https://www.st.com/resource/en/reference_manual/rm0008-.pdf) and the [M1 bench procedure](../TESTING.md#m1-hardware-acceptance).

Verify LCD controller, resolution, touch controller, bus, other peripherals and pins against schematics, markings or measurements before implementing BSP bindings. Confirm whether original firmware requires backup before flashing.

This board is the only current development, build, flash, debug and hardware acceptance target. See [platform support policy](README.md). Deferred boards do not change the STM32 resource budget or acceptance requirements.
