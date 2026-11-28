/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb
.section .text.m2_vectors,"ax",%progbits

/* Tail branches preserve EXC_RETURN. The hardware frame is 8-byte aligned
   by CCR.STKALIGN for returning handlers that use the Rust ABI. */
.global m2_external_dispatch
.thumb_func
m2_external_dispatch:
.if {test_irq}
    cmp r0, #{low}
    beq.w {low_handler}
    cmp r0, #{high}
    beq.w {high_handler}
.endif
    b m2_fault_entry

.global m2_svc
.thumb_func
m2_svc:
    b {svc}

.global m2_pendsv
.thumb_func
m2_pendsv:
    b {pendsv}

.global m2_systick
.thumb_func
m2_systick:
    b {systick}
