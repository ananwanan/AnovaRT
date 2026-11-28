# M3 memory management

M3.1 defines the board memory layout. M3.2 provides fixed-size pools. M3.3
provides TLSF. M3.4 provides stack diagnostics. M3.5 reports memory use.
M3.6 integrates boot checks and test profiles with M2. M2 is the milestone
dependency. Task creation, PSP context switching and scheduling are deferred
to their own milestones.

## SRAM budget

| Region | Design budget | M3 reservation |
|---|---:|---:|
| Static kernel and application data | 8 KiB | Linked .data and .bss |
| Emergency fault stack | 4 KiB | 4 KiB in M2 and M3 |
| Main MSP stack | 4 KiB | 4 KiB |
| Pools and future task stacks | 16 KiB | One 2 KiB pool in M3 |
| TLSF heap | 12 KiB | 12 KiB with `heap-tlsf` by default |
| Future graphics buffers | 8 KiB | 0 |
| Future device buffers | 4 KiB | 0 |
| Reserve | 8 KiB | Shared unallocated SRAM |
| Total | 64 KiB | See the ELF report for linked sizes |

Flash is `[0x08000000, 0x08080000)`; SRAM is
`[0x20000000, 0x20010000)`. The linker places .data, .bss, the M2 emergency
stack, pool and optional heap in ascending order. The main stack is at the
top of SRAM. The linker asserts bounds, alignment and non-overlap. The BSP
exposes the linker ranges through `memory::regions()`. The pool, heap and
stacks use NOBITS sections. Pool initialization writes free links and the
TLSF initializer writes headers, lists and bitmaps. Payload bytes require
initialization by their owner. The exception path initializes its own frame
as it switches to the emergency MSP and reads only data it has written there.

Reset initializes .data and .bss and paints the main stack before creating a
frame. M1 brings up clock and the optionally confirmed USART1. M2 configures
exception handling. M3 runs its boot checks in Thread Mode before M2 permits
configurable IRQ dispatch and starts SysTick. The main loop retains M2's DS0
heartbeat and M1 echo. Pool access in M3 is exclusive to the boot caller.

## Statistics

The firmware prints total SRAM, the static RAM span, pool reservation and
counts converted to bytes, the MSP snapshot and optional TLSF counters.
A pool allocation charges the entire block size. The heap reports current
requested payload bytes, occupied physical block bytes, remaining physical
free bytes, peak occupied bytes and cumulative saturating success/failure
counts. Active allocated bytes include headers and rounding. See the [TLSF
accounting rules](TLSF.md) for exactly how to interpret the heap figures.

Pool and heap controller objects are outside their backing regions. Static
RAM already includes the firmware global heap controller. Linked ranges,
runtime block utilization and logical metadata overlap by definition.
Use the ELF report to sum SRAM reservations.

See [fixed-size pools](MEMORY_POOL.md), [TLSF](TLSF.md),
[stack watermark](STACK_WATERMARK.md), and the [board procedure](../validation/M3-hardware-template.md).
