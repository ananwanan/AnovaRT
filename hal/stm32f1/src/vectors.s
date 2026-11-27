/* SPDX-License-Identifier: Apache-2.0 */
.section .vectors.external,"a",%progbits
.global ChipVectors
ChipVectors:
.rept 60                      /* STM32F103 high-density IRQs 0 through 59 */
.word DefaultHandler
.endr
