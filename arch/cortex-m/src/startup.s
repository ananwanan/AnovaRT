/* SPDX-License-Identifier: Apache-2.0 */
.syntax unified
.thumb

.section .vectors.core,"a",%progbits
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
    bl firmware_main
    b DefaultHandler
.ltorg

.section .text.DefaultHandler,"ax",%progbits
.global DefaultHandler
.thumb_func
DefaultHandler:
    cpsid i
    /* Terminal exception path. No driver, stack access, or event dependency. */
    b .
