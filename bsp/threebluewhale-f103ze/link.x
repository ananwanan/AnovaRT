/* SPDX-License-Identifier: Apache-2.0 */
ENTRY(Reset)
EXTERN(Reset ChipVectors)

MEMORY {
    FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 512K
    RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 64K
}

__stack_top = ORIGIN(RAM) + LENGTH(RAM);
__stack_bottom = __stack_top - 4K;

SECTIONS {
    .vectors ORIGIN(FLASH) : ALIGN(512) {
        KEEP(*(.vectors.core))
        KEEP(*(.vectors.external))
    } > FLASH

    .text : ALIGN(4) {
        *(.text .text.*)
        *(.rodata .rodata.*)
    } > FLASH

    .ARM.extab : { *(.ARM.extab .ARM.extab.*) } > FLASH
    .ARM.exidx : { *(.ARM.exidx .ARM.exidx.*) } > FLASH

    .data : ALIGN(4) {
        __data_start = .;
        *(.data .data.*)
        . = ALIGN(4);
        __data_end = .;
    } > RAM AT> FLASH
    __data_load = LOADADDR(.data);

    .bss (NOLOAD) : ALIGN(4) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > RAM

    .stack __stack_bottom (NOLOAD) : ALIGN(8) {
        . += 4K;
    } > RAM
}

ASSERT(SIZEOF(.vectors) == (16 + 60) * 4, "unexpected vector count");
ASSERT(ADDR(.vectors) == ORIGIN(FLASH), "vectors must start at Flash base");
ASSERT((__stack_top & 7) == 0, "initial MSP must be 8-byte aligned");
ASSERT((__stack_bottom & 7) == 0, "MSP stack base must be 8-byte aligned");
ASSERT(__bss_end <= __stack_bottom, "static RAM overlaps the MSP stack");
