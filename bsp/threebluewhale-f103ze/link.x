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
        __vectors_start = .;
        KEEP(*(.vectors.core))
        KEEP(*(.vectors.external))
    } > FLASH

    .text : ALIGN(4) {
        __text_start = .;
        *(.text .text.*)
        __text_end = .;
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

    .fault_stack (NOLOAD) : ALIGN(8) {
        __fault_stack_bottom = .;
        KEEP(*(.fault_stack))
        __fault_stack_top = .;
    } > RAM

    .memory_pool (NOLOAD) : ALIGN(32) {
        __pool_start = .;
        . += __m3_pool_bytes;
        __pool_end = .;
    } > RAM

    .memory_heap (NOLOAD) : ALIGN(8) {
        __heap_start = .;
        . += __m3_heap_bytes;
        __heap_end = .;
    } > RAM

    __sram_start = ORIGIN(RAM);
    __sram_end = ORIGIN(RAM) + LENGTH(RAM);

    .stack __stack_bottom (NOLOAD) : ALIGN(8) {
        . += 4K;
    } > RAM
}

ASSERT(SIZEOF(.vectors) == (16 + 60) * 4, "unexpected vector count");
ASSERT(ADDR(.vectors) == ORIGIN(FLASH), "vectors must start at Flash base");
ASSERT((__stack_top & 7) == 0, "initial MSP must be 8-byte aligned");
ASSERT((__stack_bottom & 7) == 0, "MSP stack base must be 8-byte aligned");
ASSERT(__fault_stack_top <= __stack_bottom, "static RAM overlaps the MSP stack");
ASSERT((__vectors_start & 511) == 0, "VTOR alignment");
ASSERT((__fault_stack_top & 7) == 0, "fault MSP alignment");

ASSERT(__data_start >= __sram_start && __data_end <= __bss_start, "static RAM layout");
ASSERT(__bss_end <= __fault_stack_bottom, "BSS overlaps fault stack");
ASSERT(__fault_stack_top <= __pool_start && __pool_end <= __heap_start, "M3 region overlap");
ASSERT(__heap_end <= __stack_bottom && __stack_top == __sram_end, "M3 regions exceed SRAM");
ASSERT((__fault_stack_bottom & 7) == 0, "fault stack base alignment");
ASSERT((__pool_start & 31) == 0 && (__pool_end & 31) == 0, "pool alignment");
ASSERT((__heap_start & 7) == 0 && (__heap_end & 7) == 0, "heap alignment");
