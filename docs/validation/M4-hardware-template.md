# M4 hardware acceptance

Use the STM32F103ZET6 Elite ME V6, BOOT0 low, SWDIO/SWCLK/ground and target
voltage reference. Identify the board and actual probe. USART1 uses 3.3 V
logic, PA9 TX and PA10 RX, 115200 baud, 8 data bits, no parity, one stop bit,
no flow control and no local echo. Enable the USART feature only after
confirming the connection. Use the already validated 8 MHz crystal input
for the HSE/PLL profile. See the [board guide](../boards/stm32f103ze.md).

## Build and program

Each download overwrites the current target firmware. Obtain operator
approval for the target and images before programming. Open serial capture
before reset, use a distinct log per profile and record the ELF hash and
Git revision.

```powershell
git rev-parse HEAD
rustc -Vv
probe-rs --version
probe-rs list
[IO.Ports.SerialPort]::GetPortNames()

$profiles = [ordered]@{
    smoke = @('m4-test-smoke,confirmed-usart1', 0)
    'hsi-no-heap' = @('m4,confirmed-usart1', 0)
    'hsi-tlsf' = @('m4,heap-tlsf,confirmed-usart1', 12288)
    'hse-no-heap' = @('m4,confirmed-hse-8mhz,confirmed-usart1', 0)
    'hse-tlsf' = @('m4,heap-tlsf,confirmed-hse-8mhz,confirmed-usart1', 12288)
    invalid = @('m4-test-invalid,confirmed-usart1', 0)
}
function Get-M4Elf([string]$name) {
    "target/m4-board/$name/thumbv7m-none-eabi/release/anova-firmware"
}
foreach ($profile in $profiles.GetEnumerator()) {
    $name = $profile.Key
    cargo build --locked --release --target thumbv7m-none-eabi -p anova-firmware `
        --features $profile.Value[0] --target-dir "target/m4-board/$name"
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $name" }
    python scripts/check_m4_elf.py (Get-M4Elf $name) --heap-bytes $profile.Value[1]
    if ($LASTEXITCODE -ne 0) { throw "ELF verification failed for $name" }
    Get-FileHash -Algorithm SHA256 -LiteralPath (Get-M4Elf $name)
}
$probe = @('--chip','STM32F103ZETx','--protocol','swd')
# Use the observed 100 kHz fallback only if default SWD is unreliable.
# $probe += @('--speed','100')
$name = 'smoke'
$elf = Get-M4Elf $name
Get-FileHash -Algorithm SHA256 -LiteralPath $elf
probe-rs download @probe --verify --reset $elf
if ($LASTEXITCODE -ne 0) { throw "Download or Verify failed for $name" }
```

The Cargo run aliases use the unchanged runner with SWD and Verify.
The separate download is useful for the already hashed, console-enabled
image. The bare aliases use default HSI and leave USART1 untouched.

## Runtime checks

| Profile | Expected observation |
|---|---|
| Smoke | 3,077 handoffs, 12 nested IRQs and 3,072 cycle samples per metric if DWT is available |
| HSI profiles | HCLK 8,000,000, SysTick reload 7,999, 1,000,007 handoffs and 3,906 nested IRQs |
| HSE profiles | HCLK 72,000,000, SysTick reload 71,999, same stress counts as HSI |
| TLSF profiles | Existing M3 allocation tests pass before M4 starts |
| Invalid | `M4 incoming context invalid`, no completion marker and no restore from the injected pointer |

The M4 PASS lines require successful start on PSP, interrupt MSP, register
and hardware frame preservation including STKALIGN padding, nested calls,
independent stacks, safe task return and recycling, controlled canary
detection and entry to idle. Confirm that the ticks increase and the worker
peaks fit their 1,024-byte stacks. A panic, fault, count mismatch or failed
Verify fails a normal run. A complete run reports `[M4 PERF]` rows when DWT
works. Without UART, use `llvm-nm -n -C <elf>` to identify `M4_COMPLETE`,
`M4_REJECTED_CONTEXT`, M2 fault record, switches and diagnostics. After idle,
`probe-rs debug @probe -c break -c 'info reg' -c 'x /1wx 0xADDRESS' -c c -c quit $elf`
can capture a result. Avoid halting the target during a timed batch.

## Performance

The fixture timestamps a request in Thread Mode before arming the sample.
The incoming task records when it reaches its first entry checkpoint or
returns through the register fixture's checks. The instrumented handoff
therefore includes Thread wrapper work, hardware exception entry and return,
and software register checks. The PendSV interval starts at its entry CYCCNT
read and ends after the restored PSP and ISB. It includes selection, checks
and measurement overhead. Save covers STMDB and adjacent counter reads.
Restore covers LDMIA, PSP write and ISB, between counter reads. Log min,
observed max, integer mean, sample count, HCLK, release size optimization,
and tick/IRQ interference. CYCCNT differences wrap modulo 32 bits. These
measurements were designed with interrupts enabled and do not establish
a worst-case execution bound. UART output occurs outside timed intervals.

## Reset and standalone

Send U after idle and verify exactly one echo. Record the post-pass DS0
heartbeat. Press physical RESET, capture a new boot and completed checks.
Disconnect SWD, remove every supply and any UART/USB back-power, then boot
in Flash mode from independent power. Capture a fresh start and PASS. Restore
the approved normal image if requested. Record board revision, commit, ELF
hash, clock, profile, Verify and reset results, attached logs and any tests
that were not run.
