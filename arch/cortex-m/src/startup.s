/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb

.section .vectors.core,"a",%progbits
.if {m2}
.word __stack_top
.word Reset
.word NmiHandler
.word HardFaultHandler
.word MemManageHandler
.word BusFaultHandler
.word UsageFaultHandler
.rept 4
.word 0
.endr
.if {m4}
.word m4_svc
.else
.word m2_svc
.endif
.word DebugMonitorHandler
.word 0
.if {m4}
.word m4_pendsv
.else
.word m2_pendsv
.endif
.word m2_systick
.else
.word __stack_top
.word Reset
.rept 5                       /* NMI through UsageFault */
.word DefaultHandler
.endr
.rept 4
.word 0
.endr
.word DefaultHandler           /* SVCall */
.word DefaultHandler           /* DebugMonitor */
.word 0
.word DefaultHandler           /* PendSV */
.word DefaultHandler           /* SysTick */

.endif

.section .text.Reset,"ax",%progbits
.global Reset
.thumb_func
Reset:
    cpsid i
    /* SAFETY: The linker bounds these disjoint, word-aligned regions.
       Hardware has loaded the aligned MSP. No Rust executes before RAM init. */
    ldr r0, =__data_load
    ldr r1, =__data_start
    ldr r2, =__data_end
1:
    cmp r1, r2
    bhs 2f
    ldr r3, [r0], #4
    str r3, [r1], #4
    b 1b
2:
    ldr r1, =__bss_start
    ldr r2, =__bss_end
    movs r3, #0
3:
    cmp r1, r2
    bhs 4f
    str r3, [r1], #4
    b 3b
4:
    .if {m3}
    /* SAFETY: No frame has been allocated. VTOR still selects the reset
       aliases and the early terminal vector does not return. */
    ldr r0, =__stack_bottom
    ldr r1, =__stack_top
    ldr r2, =0xdeadbeef
    str r2, [r0], #4
    str r2, [r0], #4
    ldr r2, =0xa5a5a5a5
5:
    str r2, [r0], #4
    cmp r0, r1
    blo 5b
    .endif
    /* Publish the physical Flash vector address once startup RAM is ready. */
    ldr r0, =0xe000ed08
    ldr r1, =__vectors_start
    str r1, [r0]
    dsb
    isb
    bl firmware_main
    b DefaultHandler
.ltorg

.section .text.DefaultHandler,"ax",%progbits
.global DefaultHandler
.thumb_func
DefaultHandler:
    .if {m2}
    mrs r0, ipsr
    subs r0, #16
    b m2_external_dispatch
    .else
    cpsid i
    /* Terminal exception path. No driver, stack access, or event dependency. */
    b .
    .endif

.if {m2}
.section .text.m2_exceptions,"ax",%progbits
.macro fault_entry name
.global \name
.thumb_func
\name:
    b m2_fault_entry
.endm
fault_entry NmiHandler
fault_entry HardFaultHandler
fault_entry MemManageHandler
fault_entry BusFaultHandler
fault_entry UsageFaultHandler
fault_entry DebugMonitorHandler

.global m2_fault_entry
.thumb_func
m2_fault_entry:
    cpsid i
    mov r1, lr
    mrs r2, msp
    mrs r3, psp
    mrs r0, ipsr
    /* At reset VTOR is zero. Do not enter Rust before RAM initialization. */
    ldr r12, =0xe000ed08
    ldr r12, [r12]
    cmp r12, #0
    beq 2f
    ldr r12, =__fault_stack_bottom
    cmp r2, r12
    blo 1f
    ldr r12, =__fault_stack_top
    cmp r2, r12
    bls 2f
1:
    /* The original frame remains in place. Use a separate aligned MSP for
       terminal diagnostics, even if original stacking failed. */
    ldr r12, =__fault_stack_top
    msr msp, r12
    b m2_fault
2:
    b 2b
.ltorg

.section .fault_stack,"aw",%nobits
.balign 8
.space 4096
.endif
