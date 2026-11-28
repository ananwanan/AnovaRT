# M3 hardware acceptance

Use the Three Blue Whales Elite ME V6 with STM32F103ZET6. Record board
revision, MCU marking, Git revision, Rust and probe-rs versions, clock,
test name, ELF SHA-256, timestamp, result and attached log for each run.
Use the [board guide](../boards/stm32f103ze.md) for pin routing and power.

## Prepare and build

Use Windows PowerShell from the repository root. Set BOOT0 to the Flash boot
position. Confirm 8 MHz HSE and USART1 routing with the operator before using
the confirmation features. Connect SWDIO, SWCLK, target reference and ground.
Connect a 3.3 V UART adapter to PA9 (board TX), PA10 (board RX) and common
ground. Record the actual COM port. If timing the DS0/PB5 heartbeat, record
the measuring instrument and its timebase accuracy.

```powershell
git rev-parse HEAD
git status --short --branch
rustc -Vv
probe-rs --version
probe-rs list
probe-rs chip info STM32F103ZETx
[IO.Ports.SerialPort]::GetPortNames()
```

After operator approval to access the board, run
`probe-rs info --protocol swd`. Match the detected STM32 family and Armv7-M
core to the physical part marking and verify the board power and BOOT0.
Use `--probe VID:PID:SERIAL` if multiple probes are present. A registered
`STM32F103ZETx` descriptor alone cannot identify the attached board.

```powershell
$profiles = [ordered]@{
    default       = @('heap-tlsf', 'tlsf')
    normal        = @('heap-tlsf,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    hsi           = @('heap-tlsf,confirmed-usart1', 'tlsf')
    'no-heap'     = @('m3,confirmed-hse-8mhz,confirmed-usart1', 'no-heap')
    pool          = @('m3-test-pool,confirmed-hse-8mhz,confirmed-usart1', 'no-heap')
    tlsf          = @('m3-test-tlsf,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    alignment     = @('m3-test-alignment,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    fragmentation = @('m3-test-fragmentation,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    oom           = @('m3-test-oom,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    stack         = @('m3-test-stack,confirmed-hse-8mhz,confirmed-usart1', 'no-heap')
    corruption    = @('m3-test-corruption,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    performance   = @('m3-test-performance,confirmed-hse-8mhz,confirmed-usart1', 'tlsf')
    'perf-hsi'    = @('m3-test-performance,confirmed-usart1', 'tlsf')
}
function Get-M3Elf([string]$profile) {
    "target/m3-board/$profile/thumbv7m-none-eabi/release/anova-firmware"
}
foreach ($entry in $profiles.GetEnumerator()) {
    cargo build --locked --release --target thumbv7m-none-eabi -p anova-firmware `
        --features $entry.Value[0] --target-dir "target/m3-board/$($entry.Key)"
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $($entry.Key)" }
    $elf = Get-M3Elf $entry.Key
    python scripts/check_m3_elf.py $elf --profile $entry.Value[1]
    if ($LASTEXITCODE -ne 0) { throw "ELF check failed for $($entry.Key)" }
    Get-FileHash -Algorithm SHA256 -LiteralPath $elf
}
```

The expected heap in these commands is 12,288 bytes. Clear any local
`ANOVART_HEAP_BYTES` override before the build matrix.

## Capture serial, flash and verify

Use a raw serial terminal at 115200 baud, eight data bits, no parity, one stop
bit, no flow control and local echo off. Start logging before flashing.
Press U once without a line ending for the echo check. To capture using a
second PowerShell session instead, enter the confirmed COM port and a distinct
log path for each profile. Press U to send a byte and Q to close the capture.

```powershell
[IO.Ports.SerialPort]::GetPortNames()
$portName = Read-Host 'Confirmed COM port'
$uartLog = Read-Host 'Unique UART log path'
$serial = [IO.Ports.SerialPort]::new($portName, 115200, 'None', 8, 'One')
$serial.Handshake = 'None'
$serial.Open()
try {
    while ($true) {
        if ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true).Key
            if ($key -eq 'Q') { break }
            if ($key -eq 'U') { $serial.Write('U') }
        }
        $chunk = $serial.ReadExisting()
        if ($chunk.Length) {
            [Console]::Write($chunk)
            [IO.File]::AppendAllText($uartLog, $chunk)
        }
        Start-Sleep -Milliseconds 20
    }
} finally {
    $serial.Dispose()
}
```

**Each download overwrites the target's firmware. Obtain explicit approval
for the named device and image before overwriting Flash.** Use the same
approved probe through all tests. Check the SHA-256 immediately before
downloading each image. Capture flashing output and UART independently.

```powershell
function Install-M3Image([string]$profile) {
    if (!$profiles.Contains($profile)) { throw "Unknown profile $profile" }
    $elf = Get-M3Elf $profile
    Get-FileHash -Algorithm SHA256 -LiteralPath $elf
    probe-rs download --chip STM32F103ZETx --protocol swd --verify --reset $elf
    if ($LASTEXITCODE -ne 0) { throw "Download, Verify or reset failed for $profile" }
}
# Run only after the operator authorizes this target and image.
Install-M3Image normal
```

For the normal image, the existing `cargo m3-run` alias also uses SWD,
Verify and `--no-catch-reset`. The download function is useful for using the
already hashed ELF from an isolated build directory.

## Expected results

Common to USART-enabled profiles are the M1 clock/console banner,
`[M3 PASS] pool`, `[M3 PASS] MSP watermark and canary`,
`[M3 PASS] memory self-test`, then M2 SysTick reload and periodic ticks.
Confirm MSP is 8-byte aligned and within the linker range with an intact
canary. Check pool use returns to zero, TLSF occupancy returns to zero when
enabled, and the reported largest free payload is heap size minus 16 bytes.
A panic, fault, missing expected UART line or Verify error fails that run.

| Profile | Operation and pass observation |
|---|---|
| default | Confirm DS0 heartbeat. USART1 remains untouched. |
| normal | Confirm HCLK=72000000, RELOAD=71999, UART echo, tick progression, and `Box Vec String and growth` PASS. |
| hsi | Confirm HCLK=8000000, RELOAD=7999, UART echo and tick progression. |
| no-heap | Confirm `TLSF disabled` with healthy ticker and echo. |
| pool | Confirm full-pool exhaustion, reuse and rejection checks complete. Final peak is 2,048 bytes and free is 2,048 bytes. |
| tlsf | Confirm `TLSF stress operations=10000 alternating=false` and `Handler Mode allocation rejected`. |
| alignment | Confirm `TLSF alignment` PASS after requests aligned from 1 to 256 bytes. |
| fragmentation | Confirm `TLSF stress operations=10000 alternating=true` and full recovery of the heap. |
| oom | Confirm exhaustion of the exact-fit arena, failed small request and successful retry after full release. |
| stack | Confirm `Stack depth` PASS, a lower deep SP, and detection of a damaged canary in a separate test buffer. |
| corruption | Confirm `TLSF malformed link and boundary detected` plus controlled canary testing and recovery. |
| performance and perf-hsi | Save all `[M3 PERF]` rows and verify the self-test and preserved heartbeat. |

Each performance row records min, observed max, integer average and sample
count. Record HCLK, release optimization, interrupt state, heap size, FL/SL,
request sizes and fragmentation scenario. The masked run includes core
operations and separately reports the counter envelope and PRIMASK wrapper.
The second run uses enabled SysTick and records delivered ticks over the
batch. This does not establish that any particular sample was interrupted.
The measured windows exclude UART output and copying or zeroing allocations.
If DWT is unavailable, the firmware reports Not Run. Any later external
timing needs an operator-confirmed pin or another instrument and a recorded
resolution. Observed maxima are not WCET.

## Reset, cold boot and restoration

Run every profile using its own hashed ELF and log. On normal, HSI and
no-heap, verify DS0 toggles around every 500 ms and M2 reports ticks every
1,000 delivered ticks. Press RESET and capture a fresh boot and echo.
Disconnect SWD and fully remove board power, including possible USB or
UART back-power, then power it in standalone Flash mode with serial logging
available. Capture a fresh boot, ticker and echo. Compare M1 clock source,
USART1, DS0 and M2 behavior against prior board evidence. Repeat any
required interrupt or architecture-fault checks using the approved images
in the [M2 hardware procedure](M2-hardware-template.md).

After tests, obtain or confirm approval for restoring the normal M3 image,
run `Install-M3Image normal`, then repeat reset and standalone checks.
Attach the result table and per-image hashes, upload logs only through the
maintainer's chosen workflow, and mark unperformed tests Not Run.
