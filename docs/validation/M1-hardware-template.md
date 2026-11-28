# M1 hardware acceptance record

Start with every row marked unverified. Copy this file to `M1-YYYY-MM-DD.md` and record actual measurements and artifacts. Follow [M1 hardware acceptance](../TESTING.md#m1-hardware-acceptance).

| Setup | Record |
|---|---|
| Operator, date and timezone | |
| Source commit and tool versions | |
| Board revision and MCU markings | |
| HSE crystal marking, schematic or frequency measurement | |
| USART1 routing, continuity, connector/jumpers, logic levels and COM port | |
| Supply, BOOT0 and SWD wiring | |
| Flash backup decision and programming authorization | |
| Firmware profile, Cargo command and ELF SHA-256 for each run | |
| Programming, verify, reset and register read commands and exit codes | |
| Scope/analyzer model, sample rate and clock accuracy | |

| Check | Expected | Status | Actual result and evidence |
|---|---|---|---|
| Confirmed HSE normal boot | PLL sourced by 8 MHz crystal, SYSCLK/HCLK 72 MHz, PCLK1 36 MHz, PCLK2 72 MHz | Unverified | |
| Normal UART bit period | About 8.68 µs, 115200 8N1 | Unverified | |
| Startup log and UART RX/TX | Board, CPU, memory, clocks, initialization and exact single-byte echo | Unverified | |
| HSE failure injection | HseTimeout, stable 8 MHz HSI nominal, working UART and DS0 | Unverified | |
| PLL failure injection | PllTimeout, stable 8 MHz HSI nominal, working UART and DS0 | Unverified | |
| SYSCLK switch failure injection | SwitchTimeout, stable 8 MHz HSI nominal, working UART and DS0 | Unverified | |
| Fallback UART bit period | About 8.63 µs nominal, within receiver tolerance | Unverified | |
| Panic with console | Message, source file, line and column, then terminal halt | Unverified | |
| Panic without console | Terminal halt with unconfigured USART1 and no resumed blink | Unverified | |
| DS0 and independent M0 | Active-low blinking in M0 and in normal and fallback M1 modes | Unverified | |
| RESET reboots normal firmware | Startup banner repeats and echo/blink resume | Unverified | |
| Cold boot without debugger | Startup banner, echo and blink after removing all power and SWD | Unverified | |

List skipped tests, failures and follow-up work. Link raw UART logs, register dumps, scope traces and any board photos used to confirm the physical connections.
