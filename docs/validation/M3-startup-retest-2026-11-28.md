# M3 startup retest

The maintainer reported successful SWD programming and Verify for
`0d5465210baeeb92dd3d5388a8d8018163c06f3338f60d4e15a96f5e03248731`.
USART1 reported HSE PLL at 72 MHz and stopped after M2 initialization.
Chip autodetection failed. Specifying STM32F103ZETx allowed programming.

The matching ELF contains `str.w r6, [sp, #0x592]` at `0x08001c02` in
TLSF global initialization. M2 had already set UNALIGN_TRP. This instruction
can fault under that configuration. No SWD fault record was supplied.
M3 now clears the trap bit while preserving other exception configuration.
Disassembly of the corrected image confirms the clear. The M2 release
disassembly still sets the trap bit.

Local checks passed with exit 0 on Windows x86_64, Rust 1.100.0.
Formatting, workspace tests and host Clippy passed. Confirmed-HSE/USART1 M3
built with the flags from the hardware procedure, passed target Clippy
and the ELF checker, and has SHA-256
`5652d49c535330baf1c31067f8370152d32dc820b41414cf41437ebba918cc47`.
Its artifact is `target/m3-board/retest-normal/thumbv7m-none-eabi/release/anova-firmware`.
M2 and M3 no-heap release and ELF checks passed.

For the original failure, if the original stopped image is still present,
read its fault record before reset or reflashing, using the address
`0x20000004` from that ELF. Then verify the replacement follows M2
initialization with M3 self-tests, SysTick and UART echo.

The maintainer subsequently reported normal running after receiving the
replacement-image procedure. No new UART transcript, flashed hash, or
fault snapshot was supplied with that report. Record this as a
maintainer-reported startup retest. Dedicated profiles, DWT measurements,
reset and standalone cold boot were pending at the time of this startup report.


## Maintainer-reported HSI performance run

The Elite ME V6 UART capture at 19:42:21–19:42:24 reports HSI at 8 MHz,
a 12,288-byte TLSF arena, FL=11, SL=8 and size optimization. All reported
M3 boot checks passed. The heap returned to zero used bytes, one free
block and 12,272 bytes of free payload. The MSP canary was intact, with
2,748 bytes estimated peak and 1,340 bytes remaining after the guard.
SysTick reported reload 7999 and ticks through 3000. The capture did not
include the flashed ELF hash, Verify output, or a UART echo exchange.

Each row below reports 128 samples in DWT cycles. PRIMASK was set in
the first batch. The later SysTick-enabled batch reported 14 delivered
ticks in total.

| Scenario | Operation | Min | Observed max | Avg |
|---|---|---:|---:|---:|
| Pool | Allocate | 75 | 75 | 75 |
| Pool | Deallocate | 86 | 86 | 86 |
| Baseline | PRIMASK entry and exit | 15 | 15 | 15 |
| Baseline | Counter envelope | 1 | 1 | 1 |
| Unfragmented | TLSF exact allocate | 247 | 247 | 247 |
| Unfragmented | TLSF isolated deallocate | 211 | 211 | 211 |
| Exhausted | TLSF OOM | 127 | 127 | 127 |
| Unfragmented | TLSF allocate with split | 397 | 397 | 397 |
| Unfragmented | TLSF deallocate with coalesce | 299 | 299 | 299 |
| Unfragmented | TLSF high alignment | 497 | 497 | 497 |
| Unfragmented | TLSF protected allocate | 432 | 439 | 435 |
| Unfragmented | TLSF protected deallocate | 322 | 322 | 322 |
| Fragmented | TLSF allocate with split | 379 | 379 | 379 |
| Fragmented | TLSF deallocate with coalesce | 302 | 302 | 302 |
| Fragmented | TLSF high alignment | 497 | 497 | 497 |
| Fragmented | TLSF protected allocate | 269 | 418 | 377 |
| Fragmented | TLSF protected deallocate | 238 | 325 | 302 |
| SysTick enabled | TLSF protected allocate | 440 | 479 | 442 |
| SysTick enabled | TLSF protected deallocate | 337 | 374 | 338 |
| SysTick enabled | Empty PRIMASK entry and exit | 17 | 53 | 17 |

These observed cycle counts apply to the captured workload.

## Hardware acceptance

On 2026-11-28 (UTC+8), the maintainer confirmed that all M3 hardware checks
passed on the Three Blue Whales Elite ME V6 / STM32F103ZET6. The confirmation
covers the remaining dedicated firmware profiles, the 72 MHz performance
run, RESET and standalone cold boot with SWD disconnected, in addition to
the earlier startup retest and HSI measurements. This is maintainer-reported
acceptance. The earlier HSI UART capture is summarized above. The maintainer
did not supply per-profile hashes, individual UART and Verify transcripts,
or numerical HSE cycle measurements with the final confirmation. The
progress record references branch snapshot `e31138f` and does not identify
the exact images flashed for those additional runs.
