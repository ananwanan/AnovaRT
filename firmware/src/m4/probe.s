/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb
.section .text.m4_probe,"ax",%progbits

.global m4_probe_registers
.thumb_func
m4_probe_registers:
    /* SAFETY: This fixture runs only on an unmasked task PSP after setup.
       Preserve ABI registers, then take PendSV with known basic and software
       frame values. All checks execute in the resumed outgoing task. */
    push {r4-r11, r12, lr}
    sub sp, #40
    str r0, [sp, #36]
    mov r4, r0
    adds r5, r4, #1
    adds r6, r4, #2
    adds r7, r4, #3
    add r8, r4, #4
    add r9, r4, #5
    add r10, r4, #6
    add r11, r4, #7
    add r1, r0, #9
    ldr r2, =0xe000ed04
    mov.w r3, #0x10000000
    add r12, r0, #12
    add lr, r0, #14
    /* Force CCR.STKALIGN padding. The resumed SP must include that word. */
    sub sp, #4
    cmp r0, r0
    str r3, [r2]
    dsb
    isb
    add sp, #4

    str r0, [sp]
    str r1, [sp, #4]
    str r2, [sp, #8]
    str r3, [sp, #12]
    str r12, [sp, #16]
    str lr, [sp, #20]
    mrs r0, apsr
    lsrs r0, #28
    cmp r0, #6
    bne 1f
    ldr r0, [sp, #36]
    cmp r4, r0
    bne 1f
    adds r0, #1
    cmp r5, r0
    bne 1f
    adds r0, #1
    cmp r6, r0
    bne 1f
    adds r0, #1
    cmp r7, r0
    bne 1f
    adds r0, #1
    cmp r8, r0
    bne 1f
    adds r0, #1
    cmp r9, r0
    bne 1f
    adds r0, #1
    cmp r10, r0
    bne 1f
    adds r0, #1
    cmp r11, r0
    bne 1f
    ldr r0, [sp]
    ldr r1, [sp, #4]
    ldr r2, [sp, #8]
    ldr r3, [sp, #12]
    ldr r12, [sp, #16]
    ldr lr, [sp, #20]
    ldr r4, [sp, #36]
    cmp r0, r4
    bne 1f
    adds r5, r4, #9
    cmp r1, r5
    bne 1f
    ldr r5, =0xe000ed04
    cmp r2, r5
    bne 1f
    cmp.w r3, #0x10000000
    bne 1f
    add r5, r4, #12
    cmp r12, r5
    bne 1f
    add r5, r4, #14
    cmp lr, r5
    bne 1f
    movs r0, #1
    b 2f
1:
    movs r0, #0
2:
    add sp, #40
    pop {r4-r11, r12, pc}
.ltorg
