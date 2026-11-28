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

## M1 hardware acceptance

Use Windows PowerShell from the repository root. First identify the board revision and confirm the 8 MHz HSE crystal and the USART1 circuit using the [board notes](boards/stm32f103ze.md). Record the source or measurement in the [M1 hardware record](validation/M1-hardware-template.md). Clock frequency and wire routing each need evidence.

Power down before wiring. Use the M0 SWD and BOOT0 setup. For a confirmed direct TTL connection, use a 3.3 V adapter and connect common ground, adapter RX to PA9 (board TX), and adapter TX to PA10 (board RX). For a verified onboard VCP, use its USB port and the board's documented jumper settings. Keep a single transmitter on PA10 and avoid contention on PA9. Use the appropriate transceiver for any connector with RS-232 levels. Identify the COM port by unplugging and reconnecting the relevant USB adapter.

```powershell
probe-rs list
$probeOptions = @('--chip','STM32F103ZETx','--protocol','swd')
probe-rs read @probeOptions b32 0xE0042000 1
[System.IO.Ports.SerialPort]::GetPortNames()
Get-CimInstance Win32_SerialPort | Select-Object DeviceID,Name
```

After confirming both board inputs, build separate ELF files. Stop if any build exits nonzero. Switch and PLL injections require a working HSE so boot reaches the relevant step. The injected switch failure deliberately enters PLL first, then executes recovery from that active source.

```powershell
$confirmed = 'confirmed-hse-8mhz,confirmed-usart1'
$profiles = [ordered]@{
  normal = 'm1-build'
  hse = 'm1-hse-fallback-build'
  pll = 'm1-pll-fallback-build'
  switch = 'm1-switch-fallback-build'
  panic = 'm1-panic-build'
}
foreach ($profile in $profiles.Keys) {
  cargo $profiles[$profile] --features $confirmed --target-dir "target/m1-$profile"
  if ($LASTEXITCODE -ne 0) { throw "Build failed for $profile" }
}
cargo m1-panic-build --target-dir target/m1-panic-no-console
if ($LASTEXITCODE -ne 0) { throw 'Build failed for panic-no-console' }
cargo m0-build --target-dir target/m0-regression
if ($LASTEXITCODE -ne 0) { throw 'Build failed for M0' }
```

**Stop before programming and obtain operator approval. Programming overwrites Flash sectors.** Confirm the backup requirement. If needed and permitted, read Flash and record its hash before programming.

```powershell
probe-rs read @probeOptions b8 0x08000000 524288 --format binary --output target/original-flash.bin
if ($LASTEXITCODE -ne 0) { throw 'Backup read failed' }
Get-FileHash target/original-flash.bin
```

After approval, program and verify the selected profile. Repeat with `hse`, `pll`, `switch`, `panic`, and `panic-no-console`. For M0 use `target/m0-regression/thumbv7m-none-eabi/release/anova-firmware`.

```powershell
$profile = 'normal'
$image = "target/m1-$profile/thumbv7m-none-eabi/release/anova-firmware"
Get-FileHash $image
probe-rs download @probeOptions --verify $image
if ($LASTEXITCODE -ne 0) { throw 'Programming or verification failed' }
probe-rs reset @probeOptions
if ($LASTEXITCODE -ne 0) { throw 'Reset failed' }
```

Run this in a second PowerShell window with the verified COM port and then reset the board in the first window or press RESET. Record the output before proceeding. For panic profiles omit the echo block. Close other applications using the same COM port.

```powershell
$portName = Read-Host 'Verified COM port'
$serial = [System.IO.Ports.SerialPort]::new($portName, 115200, 'None', 8, 'One')
$serial.Handshake = 'None'
$serial.ReadTimeout = 2000
$serial.WriteTimeout = 2000
$serial.Open()
try {
  Read-Host 'Press board RESET, then press Enter to capture startup'
  Start-Sleep -Milliseconds 250
  $serial.ReadExisting() | Tee-Object -FilePath target/m1-uart.txt -Append
  $serial.Write('U')
  $received = $serial.ReadByte()
  if ($received -ne 0x55) { throw "Unexpected echo byte $received" }
  'Confirmed single-byte echo 0x55'
} finally {
  $serial.Dispose()
}
```

Read the live RCC, Flash ACR and USART1 BRR after a normal and each fallback boot. With multiple probes, add the intended `--probe VID:PID:Serial` to `$probeOptions`. Check native exit codes and save raw output.

```powershell
probe-rs read @probeOptions b32 0x40021000 2
probe-rs read @probeOptions b32 0x40022000 1
probe-rs read @probeOptions b32 0x40013808 1
```

1. Normal boot must show `HsePll`, 72/72/36/72 MHz and startup memory, DS0 and console status. Check RCC_CFGR SWS=2, PLLSRC=1, PLLXTPRE=0, PLLMUL=7, HPRE=0, PPRE1=4 and PPRE2=0. Check ACR LATENCY=2 and PRFTBE=1. Establish SYSCLK from the confirmed crystal and register values. Independently measure the PA9 bit period on the scope or a suitably fast logic analyzer using the echoed `U` pattern. Expect approximately 8.68 µs with BRR=625. Measured baud × BRR gives PCLK2, which equals HCLK and SYSCLK in this profile.
2. Each injected recovery must report its specific reason and nominal 8/8/8/8 MHz. Check SWS=0 and all three bus dividers /1, HSE and PLL off, and zero Flash wait states. Check DS0 and exact single-byte echo. BRR=69 gives about 8.63 µs nominal. Record measured baud and any HSI tolerance effect.
3. Confirm panic with the console reports `M1 panic diagnostic test` and the file, line and column. The startup LED state should hold. For the no-console image, check that the USART1 clock remains disabled and DS0 does not resume blinking. If needed use a debugger to verify the PC is parked in the architecture halt loop using the corresponding ELF disassembly.
4. Confirm DS0 is active-low and blinks for at least three complete cycles in normal M1, each fallback and the independent M0 image.
5. Reflash normal M1 and repeat banner, echo and LED checks after the board RESET button. With power off, detach SWD and remove every source of board power, including any onboard USB connection that supplies it. Restore power and repeat with a serial-only connection. Capture the banner on cold boot with the serial receiver ready or record it on the analyzer.
6. Complete the [hardware record](validation/M1-hardware-template.md) and update both progress files after all checks pass.

## M2 interrupts and exceptions

Host tests cover IRQ bank arithmetic and limits, priority encoding, SysTick
reload, wraparound, EXC_RETURN, stack validation and fault status flags.
Validation firmware checks nested PRIMASK and BASEPRI restoration,
priority ordering, active/pending readback, and SVC/PendSV exception return.
These runtime checks require the STM32F103ZET6. Run `scripts/check_m2_elf.py`
on each target profile and use LLVM objdump to inspect `m2_fault_entry` and
the returning veneers. Keep the tested ELF SHA-256 with its serial log and
measurement. Follow the [M2 hardware procedure](validation/M2-hardware-template.md).


## M3 memory verification

Run workspace host tests, Clippy and the STM32 release build. The memory
tests exercise all fixed-block classes, alignment through 256 bytes,
physical and free-list consistency, and 100,000 fixed-seed heap operations
with payload integrity checks. Also build and inspect `m3-no-heap-build`
and each `m3-test-*-build` profile. Build isolates are under `target/m3*`.
`scripts/check_m3_elf.py` reuses the vector and load-segment checks from M2.
Use `--profile legacy` for M0/M1, `m2` for M2, `no-heap` for heap-free M3,
and `tlsf` when the global allocator is linked.

The [M3 hardware procedure](validation/M3-hardware-template.md) covers
UART and DS0, bounded on-board stress tests, DWT, reset and cold boot.
Keep software checks, per-image SWD verification and runtime observations
in separate evidence fields.
