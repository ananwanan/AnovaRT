/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb
.section .text.m4_switch,"ax",%progbits

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
    stmdb r1!, {{r4-r11}}
    ldmia r0!, {{r4-r11}}
    msr psp, r0
    isb
2:
    bx lr

m4_bad_return:
    udf #4
.ltorg
