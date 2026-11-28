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
reset and standalone cold boot still require evidence.
