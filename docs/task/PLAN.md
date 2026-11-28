# M4 implementation plan

M0–M3 are recorded as done on dev. M4 keeps the single STM32F103ZET6 target,
the existing clock, interrupt and allocator profiles, and the M3 memory map.

| Stage | Dependency | Implementation and local evidence | Board acceptance |
|---|---|---|---|
| M4.1 | M3 | Bounded generational task table and state transitions, host tests | Exercise creation and stale IDs |
| M4.2 | M4.1 | Exclusive static stack loans, bounds and M3 guard reuse, host tests | Verify independent stacks and watermark |
| M4.3 | M4.2 | Cortex-M3 basic frame and Thumb entry encoding, host layout tests | Verify initial argument and return |
| M4.4 | M4.3 | SVC bootstrap on MSP and first PSP restore, ELF check | Verify Thread and Handler stack selection |
| M4.5 | M4.4 | PendSV save/restore and checked context selection, ELF check | Verify registers and IRQ coexistence |
| M4.6 | M4.5 | Cooperative switch and exit with separate idle and deferred reclamation | Verify task return, reuse and idle |
| M4.7 | M4.6 | Register fixture, stress, DWT and STM32 build matrix | Complete both clock and heap configurations |

The static stack API takes exclusive owned loans. The TLSF-enabled image retains
M3 allocator validation. Dynamically allocated task stacks are optional and
deferred. M5 owns preemption, ready queues, blocking and timing policy. MPU
isolation and secondary platforms are outside this milestone.
