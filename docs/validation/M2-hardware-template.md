# M2 hardware acceptance

For the milestone closeout, see the [hardware acceptance record](M2-2026-11-28.md).
Record observations from the specified image on the Three
Blue Whales Elite ME V6 with STM32F103ZET6. Expected output below is a pass
criterion. Use the [board guide](../boards/stm32f103ze.md) for wiring.

## Prepare and build

Use Windows PowerShell from the repository root. Set BOOT0 for Flash boot,
connect ST-Link/SWD with ground and target reference, and verify the board
revision and clock before using the confirmation flags. Connect a 3.3 V
USART1 adapter to PA9 (board TX), PA10 (board RX) and ground. Close other
programs holding the COM port. Attach an independently clocked logic analyzer
or oscilloscope to DS0/PB5 with a common ground. Record the instrument's time
base accuracy and keep the captures for both clock profiles.

```powershell
git rev-parse HEAD
git status --short
rustc -Vv
probe-rs --version
probe-rs list
$flags = 'confirmed-hse-8mhz,confirmed-usart1'
$profiles = [ordered]@{
    normal = 'm2-build'
    nvic = 'm2-test-nvic-build'
    critical = 'm2-test-critical-build'
    svc = 'm2-test-svc-build'
    wrap = 'm2-test-wrap-build'
    usage = 'm2-fault-usage-build'
    bus = 'm2-fault-bus-build'
    hard = 'm2-fault-hard-build'
    irq = 'm2-fault-irq-build'
    silent = 'm2-fault-silent-build'
    m1 = 'm1-build'
    m1panic = 'm1-panic-build'
}
function Get-M2Elf([string]$Profile) {
    "target/m2-board/$Profile/thumbv7m-none-eabi/release/anova-firmware"
}
foreach ($entry in $profiles.GetEnumerator()) {
    cargo $entry.Value --features $flags --target-dir "target/m2-board/$($entry.Key)"
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $($entry.Key)" }
}
cargo m2-build --features "m1-fault-hse,$flags" --target-dir target/m2-board/hsi
if ($LASTEXITCODE -ne 0) { throw 'HSI build failed' }
cargo m2-build --target-dir target/m2-board/default
if ($LASTEXITCODE -ne 0) { throw 'Default build failed' }
cargo m0-build --target-dir target/m2-board/m0
if ($LASTEXITCODE -ne 0) { throw 'M0 build failed' }
$profiles.Keys + @('hsi','default','m0') | ForEach-Object {
    Get-FileHash -Algorithm SHA256 -LiteralPath (Get-M2Elf $_)
}
```

If the probe does not enumerate, check USB, drivers and device ownership. If
enumeration succeeds but attach fails, check board power, SWDIO, SWCLK,
ground and target reference. Check NRST wiring before trying connect under
reset. Keep BOOT0 in its normal position for acceptance.

## Capture serial before reset

Use Serial Studio or a serial terminal in raw mode at 115200 8N1, flow
control off and local echo off. Press U without a newline for the echo
check. Alternatively start this PowerShell capture in a second terminal.
Replace COM3 with the port shown by the first command and choose a distinct
log name for each image. This loop sends one U when U is pressed and logs
only received data. Press Q to close the port.

```powershell
[IO.Ports.SerialPort]::GetPortNames()
$port = [IO.Ports.SerialPort]::new('COM3', 115200, 'None', 8, 'One')
$port.Handshake = 'None'
$port.Open()
try {
    while ($true) {
        if ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true).Key
            if ($key -eq 'Q') { break }
            if ($key -eq 'U') { $port.Write('U') }
        }
        $chunk = $port.ReadExisting()
        if ($chunk.Length -gt 0) {
            [Console]::Write($chunk)
            [IO.File]::AppendAllText((Join-Path (Get-Location) 'm2-profile-uart.log'), $chunk)
        }
        Start-Sleep -Milliseconds 20
    }
} finally {
    $port.Dispose()
}
```

## Flash, inspect and restore

**Each download overwrites the current firmware in Flash. Obtain the
operator's explicit approval and confirm the selected board and image
before running a download.** Start serial capture first, then use the
function with the profile from the table. Hash the chosen ELF immediately
before download. A nonzero programming, verification or reset exit code
invalidates the run.

```powershell
function Install-M2Profile([string]$Profile) {
    $elf = Get-M2Elf $Profile
    Get-FileHash -Algorithm SHA256 -LiteralPath $elf
    probe-rs download --chip STM32F103ZETx --protocol swd --binary-format elf --verify $elf
    if ($LASTEXITCODE -ne 0) { throw "Download or verification failed for $Profile" }
    probe-rs reset --chip STM32F103ZETx --protocol swd
    if ($LASTEXITCODE -ne 0) { throw "Reset failed for $Profile" }
}

Install-M2Profile normal

# Normal runtime: VTOR, AIRCR and CCR, then SysTick CTRL/LOAD/VAL.
probe-rs read --chip STM32F103ZETx --protocol swd b32 0xe000ed08 3
probe-rs read --chip STM32F103ZETx --protocol swd b32 0xe000e010 3
# Terminal image: ICSR and SHCSR/CFSR/HFSR/DFSR/MMFAR/BFAR/AFSR.
probe-rs read --chip STM32F103ZETx --protocol swd b32 0xe000ed04 1
probe-rs read --chip STM32F103ZETx --protocol swd b32 0xe000ed24 7

# Set the profile to the terminal image actually on the board.
$profile = 'silent'
$llvm = Join-Path (rustc --print sysroot) 'lib/rustlib/x86_64-pc-windows-msvc/bin'
$line = & (Join-Path $llvm 'llvm-nm.exe') --defined-only (Get-M2Elf $profile) |
    Where-Object { $_ -match ' M2_FAULT_RECORD$' }
if (-not $line) { throw 'Fault record symbol missing' }
$address = '0x' + ($line.Trim() -split '\s+')[0]
probe-rs read --chip STM32F103ZETx --protocol swd b32 $address 22

# Restore the normal profile after each diagnostic test.
Install-M2Profile normal
```

SysTick CTRL reads consume COUNTFLAG. Perform SWD register checks separately
from timing captures. The fault record has 22 words: exception number,
EXC_RETURN, saved MSP, saved PSP, CFSR, HFSR, DFSR, AFSR, MMFAR, BFAR, SHCSR,
ICSR, frame-valid flag, R0, R1, R2, R3, R12, LR, PC, xPSR and completion
marker `0x4d324641`. Use MMFAR and BFAR only when the corresponding CFSR valid
bit is set. A zero frame-valid flag means the eight frame words are unavailable.
A missing completion marker means capture did not finish.

## Execute and record

For each row use its named build and `Install-M2Profile <profile>`. Start
capture before installation. Save the raw log and note observed LED
behavior, register data and the ELF hash. After any terminal case, preserve
the record before reset. Restore `normal` and verify its banner, DS0 and
echo before proceeding. Treat a panic assertion, unexpected reset, missing
required marker or a verification error as failure.

| Profile | Purpose and steps | Passing observation | Troubleshoot |
|---|---|---|---|
| `normal` | Use the confirmed board wiring and independent timer. Start the serial and PB5 captures, install, send one U and capture at least 20 consecutive DS0 edge intervals. | Banner shows PLL, HCLK 72000000 and RELOAD 71999. One U is echoed. Tick lines continue. DS0 alternates with nominal 500 ms edges and 1000 ms full cycles. VTOR is 08000000 and AIRCR PRIGROUP is 3. | Check HSE/PLL and actual LOAD, serial wiring and probe disturbance. A fallback banner fails this row. |
| `hsi` | Use the same setup and repeat the capture and echo. This profile injects an HSE timeout using M1's clock-recovery path. | HSI fallback is reported, HCLK is nominally 8000000 and RELOAD is 7999. One U is echoed and DS0 alternates with nominal 500 ms edges within the HSI error budget. | Check flash hash, RCC clock source and analyzer input. Verify that the HSE confirmation flag accompanied fault injection. |
| `default` | Build without either confirmation flag, install and observe DS0. | Continuous DS0 edges at nominal HSI timing. No USART1 output. SWD LOAD is 7999. | Confirm the ELF came from the default directory. |
| `nvic` | Use confirmed USART1. Install and wait for the status line. No peripheral stimulus is required. | `[M2 PASS] NVIC` includes `0x312` and `0x3241`. The profile checks enable/disable, pending/clear, priority readback, active nesting, dispatch after unmask and both directions of preemption. Normal DS0 blink and tick logs follow. | Inspect PRIGROUP and NVIC priorities for IRQs 11 and 12. A panic assertion identifies the first failing check. |
| `critical` | Use confirmed USART1. Install and wait for the status line. | `[M2 PASS] critical: PRIMASK BASEPRI IPSR` appears. The profile checks nested incoming masks, stronger/weaker/zero ceilings, dispatch across the threshold, IPSR and state restoration. Normal blink and tick logs follow. | Inspect the panic location and disassemble PRIMASK, BASEPRI and BASEPRI_MAX writes. |
| `svc` | Use confirmed USART1. Install and wait for the status line. | `[M2 PASS] SVC/PendSV: returned to Thread Mode` appears after vectors report IPSR 11 and 14. Tick logs and DS0 blink continue. | Inspect SVC and PendSV vector addresses and whether a debugger intercepted the exception. |
| `wrap` | Use confirmed USART1 and start PB5 capture before reset. | `[M2 PASS] tick wrap` appears once, periodic lines continue with low unsigned tick values, and no DS0 interval is skipped at the wrap. | Check the image hash and retain the first two DS0 transitions across reset and wrap. |
| `usage` | Use confirmed USART1. Install and capture the terminal record. | UsageFault, IPSR 6, UNDEFINSTR (CFSR bit 16), PC at the UDF instruction, valid frame and system halt. DS0 stays inactive. | Verify SHCSR UsageFault enable and that SWD did not intercept the fault. |
| `hard` | Use confirmed USART1. Install and capture the terminal record. | HardFault, IPSR 3, HFSR.FORCED and CFSR.UNDEFINSTR, valid frame with UDF PC and system halt. DS0 stays inactive. | Check the profile hash and SHCSR UsageFault disable. |
| `bus` | Use confirmed USART1. Install and capture the terminal record. The profile issues one aligned read from reserved peripheral address `0x400f0000`. | BusFault, IPSR 5, CFSR.PRECISERR, BFAR matching the probe address if BFARVALID is set, and system halt. DS0 stays inactive. | If the fabric returns normally, the firmware reports a panic. A panic or lack of response does not pass. Preserve logs and report the silicon revision and SCB state. |
| `irq` | Use confirmed USART1. Install and capture the terminal record. | Unexpected IRQ, IPSR 29 (external IRQ 13), valid frame and system halt. DS0 stays inactive. | Check the ICSR active exception and verify the intended vector/image. |
| `silent` | Connect SWD, retain serial capture and install. The profile skips USART1 setup even with confirmation flags. Read the SRAM record before resetting. | No UART data, DS0 inactive, complete marker, IPSR 6 and CFSR.UNDEFINSTR. No restart. | Check image hash and record symbol from the exact ELF. A dark LED alone does not establish fault entry. |
| `m1panic` | Install the independent M1 panic image with confirmed USART1. | M1 banner and `[PANIC]` with `M1 panic diagnostic test`. DS0 stays inactive. | Check that the independent M1 profile was installed and that the boot banner was captured. |
| `m1` | Install the independent M1 image with confirmed board setup. Repeat the M1 hardware behavior checks, including a U echo. | Startup and clocks banner, DS0 blink and one-byte echo. | Follow the [M1 hardware procedure](M1-hardware-template.md). |
| `m0` | Install the independent M0 image and observe repository firmware. | DS0 blinks after the startup self-check. UART remains silent. | Check the Flash boot selection and follow the [M0 hardware procedure](M0-hardware-template.md). |
| `normal` after terminal tests | Restore normal, reset with SWD, then disconnect ST-Link and fully power-cycle the board with UART capture already listening. Avoid powering the board through debug or UART signal pins. | The banner, DS0 timing, tick and U echo return after reset and after a debugger-free cold boot. | Verify supply, BOOT0 and stable UART ground. Restore normal again if a test image was left in Flash. |

Calculate frequency from the edge count using
`tick_hz = 500 * edge_intervals / elapsed_seconds`. Record interval count,
elapsed measurement, mean and worst edge interval, measurement uncertainty
and the accepted clock tolerance. Account for crystal accuracy or HSI
calibration tolerance as applicable, plus Thread Mode sampling jitter. Use
the applicable STM32F103xC/D/E datasheet conditions for HSI tolerance and
note supply and temperature. Supply the raw capture and explain any outlier.
Serial tick counts alone do not measure the oscillator.

If possible, inspect the lowest observed MSP during terminal formatting
and during nested IRQs in separate debug runs. Check that each remains in
its corresponding 4 KiB reservation. Spot checks do not establish a stack
high-water mark.

MemManage, NMI, DebugMonitor, invalid state/PC, divide-by-zero,
unaligned access, imprecise bus errors, stacking/unstacking faults,
vector-fetch errors and lockup have no dedicated board acceptance profile
in M2. Record them as unverified unless separately tested on the device.
Host flag decoding and checked frame arithmetic are static/host evidence.

## Evidence to fill in

| Item | Observation |
|---|---|
| Operator, date, board revision, MCU marking and supply | |
| Commit, worktree status, tools and configuration | |
| SWD probe, port, oscillator and UART wiring | |
| Independent time reference, uncertainty and clock tolerance | |
| Each build profile, ELF SHA-256 and verify/reset results | |
| Per-profile raw UART log, LED observation and pass/fail | |
| 72 MHz and HSI timing captures and measured values | |
| Terminal SCB/snapshot and stack evidence | |
| Reset and debugger-free cold boot | |
| Failures, retest evidence and unverified cases | |
| Normal image restoration and final approval | |

Save the completed record separately under `docs/validation/`. Once all
required criteria and M1 dependency evidence are satisfied, update
`docs/progress.json` and `docs/PROGRESS.md` together with the hardware
evidence and run `python scripts/check_progress.py`.
