/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb
.section .text.m4_switch,"ax",%progbits

/* Snapshot only when the firmware has positively probed DWT. The sample
   block is owned by PendSV and read by Thread Mode between requests. */
.macro stamp offset
    ldr r12, =m4_cycle_trace
    ldr r2, [r12]
    cbz r2, 9f
    ldr r2, =0xe0001004
    ldr r3, [r2]
    str r3, [r12, #\offset]
9:
.endm

.global m4_svc
.thumb_func
m4_svc:
    /* SAFETY: Only bootstrap from privileged Thread/MSP. Rust runs on the
       aligned MSP and returns a checked, nonzero saved context. */
    ldr r0, ={thread_msp}
    cmp lr, r0
    bne m4_bad_return
    bl m4_first
    ldmia r0!, {{r4-r11}}
    msr psp, r0
    /* Abandon both the startup call chain and its hardware frame. No Rust
       frame survives this instruction. Mask configurable IRQs for the commit. */
    cpsid i
    ldr r1, =__stack_top
    msr msp, r1
    ldr lr, ={thread_psp}
    dsb
    isb
    cpsie i
    bx lr

.global m4_pendsv
.thumb_func
m4_pendsv:
    stamp 4
    ldr r0, ={thread_psp}
    cmp lr, r0
    bne 2f
    /* SAFETY: AAPCS preserves outgoing r4-r11 across the selector. It checks
       space for software and hardware frames before the first PSP store.
       The MSP push keeps alignment and preserves the EXC_RETURN token. */
    mrs r0, psp
    push {{r0, lr}}
    bl m4_select
    pop {{r1, lr}}
    cbz r0, 2f
    /* Time the transfers individually. CYCCNT and trace are Device and
       normal memory respectively, both accessed as aligned words. */
    ldr r12, =m4_cycle_trace
    ldr r2, [r12]
    cbz r2, 1f
    ldr r2, =0xe0001004
    ldr r3, [r2]
    stmdb r1!, {{r4-r11}}
    ldr r1, [r2]
    str r3, [r12, #8]
    str r1, [r12, #12]
    ldr r3, [r2]
    ldmia r0!, {{r4-r11}}
    msr psp, r0
    isb
    ldr r1, [r2]
    str r3, [r12, #16]
    str r1, [r12, #20]
    bx lr
1:
    stmdb r1!, {{r4-r11}}
    ldmia r0!, {{r4-r11}}
    msr psp, r0
    isb
2:
    bx lr

m4_bad_return:
    udf #4
.ltorg

.section .bss.m4_cycle_trace,"aw",%nobits
.balign 4
.global m4_cycle_trace
m4_cycle_trace:
.space 24
