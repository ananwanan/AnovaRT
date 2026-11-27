# Experimental LCD pixel cat demo

Status: implemented as an independent M11 precursor; build and hardware acceptance pending. This is not the M11 driver or touch implementation, and no formal milestone status changes.

## Preconditions (verify before flashing)

The only target is the Three Blue Whales Elite ME V6 STM32F103ZET6. The following is a candidate wiring based on similar Elite boards, not a verified binding for this board. Confirm the exact LCD module model, voltage, controller (ILI9341), 240x320 resolution, 16-bit 8080 interface, reset connection and backlight polarity against the module markings/schematic and board schematic or continuity measurements. Do not run the LCD firmware on an unverified wiring or a different controller.

| Signal | Candidate |
|---|---|
| CS | PG12 / FSMC_NE4 |
| RS / DC | PG0 / FSMC_A10 |
| WR | PD5 / FSMC_NWE |
| RD | PD4 / FSMC_NOE |
| D0-D3 | PD14, PD15, PD0, PD1 |
| D4-D12 | PE7 through PE15 |
| D13-D15 | PD8, PD9, PD10 |
| BL | PB0, active high |
| RESET | Board reset or module-provided reset; software reset also used |

Reference for similar boards (not evidence for the actual Elite ME V6): https://www.iotword.com/7189.html

The 16-bit bus shifts CPU address bit 11 to FSMC A10: command 0x6C000000, data 0x6C000800. The demo uses conservative asynchronous FSMC timings and uncalibrated busy waits. It renders directly into panel GRAM, without a full framebuffer or heap. A full RGB565 framebuffer would need 153600 bytes, more than the 64 KiB SRAM.

## Build and flash

From this branch, run:

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
cargo m0-build
cargo lcd-cat-build
cargo clippy --locked --release --target thumbv7m-none-eabi -p anova-firmware --features lcd-cat -- -D warnings
python scripts/check_progress.py
```

The normal M0 firmware is still the default. LCD MMIO is accessed only when the explicit lcd-cat feature is selected at build time and that firmware is executed.

After confirming the wiring and approving Flash overwrite (back up the original firmware if required):

```powershell
cargo lcd-cat-run
```

Alternatively build with cargo lcd-cat-build and use probe-rs download --chip STM32F103ZETx --protocol swd --verify target/thumbv7m-none-eabi/release/anova-firmware, then probe-rs reset --chip STM32F103ZETx --protocol swd. Note that the LCD and M0 aliases share the executable path; rebuild the intended variant immediately before manual flashing.

## Acceptance

1. Backlight turns on after initialization.
2. Red, green, blue, white and black fill the whole visible panel in sequence.
3. The final pink background, black-and-white pixel cat, two text lines and eight RGB565 color bars appear without clipping or corruption.
4. Disconnect the debugger, reset and power-cycle; the sequence repeats from Flash.
5. Restore the M0 firmware with cargo m0-run if needed; confirm the DS0 blink returns.

If the screen stays white or dark, stop and verify the module and wiring before changing controller initialization. Red/blue swaps suggest MADCTL color order, wrong orientation suggests MADCTL scan direction, and corruption may indicate data lines or FSMC timing. Do not probe arbitrary board pins. No touch, DMA, clock-tree changes or RTOS integration are included.

Record commit, board/module markings, wiring evidence, commands, exit codes, tool versions, photos/video of the display, reset and power-cycle results, and any failures in a separate validation record. Formatting, host tests, cross-build, Clippy and hardware tests were not run in the authoring environment; no Rust toolchain or physical board was available.
