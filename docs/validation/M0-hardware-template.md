# M0 hardware acceptance record

Copy to `M0-YYYY-MM-DD.md` and fill in the observed results.

- Date, time zone, operator:
- Firmware commit and workspace status:
- ELF path and SHA-256:
- Host OS, Rust, Cargo and probe-rs versions:
- Board revision, MCU marking, probe model and firmware version:
- Flash overwrite approval and backup disposition:
- BOOT0 position and SWD wiring:
- Flash bytes, static SRAM bytes and reserved MSP stack bytes:
- Evidence paths or URLs:
- Tests not run and reasons:

| Check | Command or method | Exit code | Expected | Observed and evidence |
|---|---|---|---|---|
| Probe and chip identification | `probe-rs list`, `probe-rs info --chip STM32F103ZETx --protocol swd` | | Probe detected and Cortex-M3 identified | |
| Flash and readback | `probe-rs download --chip STM32F103ZETx --protocol swd --verify <ELF>` | | Verified success | |
| Reset and blink | `probe-rs reset --chip STM32F103ZETx --protocol swd` | | DS0 blinks repeatedly | |
| Pin and polarity | Correlate PB5 with DS0 using a scope or debugger | N/A | PB5 low lights DS0, PB5 high extinguishes DS0 | |
| Cold boot | Power-cycle with BOOT0 low and debugger detached | N/A | DS0 resumes blinking from Flash | |

Record the final pass or failure and update the board document with the observed DS0 mapping and polarity.
