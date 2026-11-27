# Testing and acceptance

Active scope: the Three Blue Whales STM32F103ZET6 board only. Layers: host unit tests for portable logic; thumbv7m-none-eabi cross-builds; hardware SWD, UART and GPIO verification; logic analyzer for timing when needed. QEMU for active-target work, if applicable, is supplemental, not a substitute for board verification.

Record commit SHA, date, tool versions, board, command, exit code, expected and actual results, evidence path or URL and tests not run. Store records in docs/validation/Mn-YYYY-MM-DD.md. Each evidence item in progress.json contains commit, environment, result (pass) and artifact. Mark done only when all acceptance points pass. Without hardware, use implemented or blocked rather than fabricated evidence.

Scheduling tests cover preserved registers, stack boundaries, preemption order, concurrency, timeouts, tick wraparound, ISR latency and context-switch timing. Report measurement conditions and maxima; do not claim hard real-time performance from averages. Memory tests cover linker map, stack watermark, OOM, bounds and fragmentation. Driver tests cover errors, timeouts and bounded retries; use backed-up test media for storage.

## Secondary platforms and release gates

Raspberry Pi 4B and other deferred boards are outside the current verification scope. Do not run secondary cross-builds, QEMU or other simulations, hardware boot, flashing, debugging, peripheral tests, performance tests, or secondary CI matrix jobs. Record their status as deferred / not run, not as pass, fail or blocked for an active milestone. Host tests of portable logic do not establish secondary-board compatibility. No secondary target gates STM32 milestones or releases. M22 is satisfied by the documented portability review and applicable active-target regression evidence, without second-board execution. A future port requires its own explicit activation and validation plan before claiming support.


## M0 hardware acceptance

Use the Three Blue Whales Elite ME V6 with STM32F103ZET6, the correct board supply and BOOT0 low. With power off, connect SWDIO, SWCLK and common ground, plus target voltage reference as required by the probe. Match pin labels on the board and probe. Power the board before identifying the chip.

```powershell
cargo m0-build
probe-rs list
probe-rs info --chip STM32F103ZETx --protocol swd
```

**Stop before programming and obtain operator approval. Programming overwrites Flash sectors used by the firmware.** Confirm whether the existing firmware must be backed up. If more than one probe is attached, select the intended probe with `--probe VID:PID:Serial`.

After approval, run `cargo m0-run` to program, verify and launch under the runner, or use the following commands to record programming and reset separately. Proceed to reset only after verified programming succeeds.

```powershell
probe-rs download --chip STM32F103ZETx --protocol swd --verify target/thumbv7m-none-eabi/release/anova-firmware
probe-rs reset --chip STM32F103ZETx --protocol swd
```

1. Observe at least three complete DS0 cycles. The busy wait has no calibrated timing requirement.
2. Correlate the PB5 level with DS0 using a scope or logic analyzer and visual observation. Alternatively, halt with an SWD debugger in each delay loop and read GPIOB_IDR bit 5 at `0x40010C08`. Low should coincide with DS0 on, and high with DS0 off.
3. Disconnect the debugger and power-cycle the board. Confirm that DS0 resumes blinking from Flash.
4. Fill out the [hardware record](validation/M0-hardware-template.md). Record the actual pin and polarity in the [board document](boards/stm32f103ze.md), with a link to the completed record. Update both progress files after acceptance.

The [local ELF record](validation/M0-local-2026-11-28.md) covers the build and memory layout. It also shows how to reproduce the inspection.
