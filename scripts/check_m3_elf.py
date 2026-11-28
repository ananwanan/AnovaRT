# SPDX-License-Identifier: Apache-2.0
"""Verify M3 linker regions and report STM32F103ZET6 resource usage."""

import argparse
import struct
from pathlib import Path

from check_m2_elf import require, verify


def check_arm_profile(elf, header):
    """Read the ARM EABI file attributes through Tag_CPU_arch_profile."""
    require(header is not None, "Missing ARM build attributes")
    section = elf[header[4]:header[4] + header[5]]
    require(section[:1] == b"A", "Unexpected ARM build attributes format")
    start = 1
    while start + 4 <= len(section):
        length, = struct.unpack_from("<I", section, start)
        end = start + length
        require(length >= 5 and end <= len(section), "Invalid ARM vendor attribute length")
        name_end = section.find(b"\0", start + 4, end)
        require(name_end >= 0, "Unterminated ARM attribute vendor")
        if section[start + 4:name_end] == b"aeabi":
            break
        start = end
    else:
        raise SystemExit("Missing ARM EABI vendor attributes")

    cursor = name_end + 1
    require(cursor + 5 <= end and section[cursor] == 1, "Missing ARM file attributes")
    length, = struct.unpack_from("<I", section, cursor + 1)
    file_end = cursor + length
    require(length >= 5 and file_end <= end, "Invalid ARM file attribute length")
    cursor += 5
    values = {}
    while cursor < file_end:
        tag = section[cursor]
        cursor += 1
        if tag in (4, 5, 67):
            end_string = section.find(b"\0", cursor, file_end)
            require(end_string >= 0, "Unterminated ARM string attribute")
            cursor = end_string + 1
        elif tag in (6, 7, 8, 9):
            require(cursor < file_end and section[cursor] < 128, "Invalid ARM ISA file attribute")
            values[tag] = section[cursor]
            cursor += 1
            if tag == 9:
                break
        else:
            break
    require((values.get(6), values.get(7), values.get(9)) == (10, ord("M"), 2),
            "Expected ARMv7-M with Thumb-2")


def check(path, profile, expected_heap, task_context=False):
    heap_enabled = profile == "tlsf"
    sections, symbols, flash_segments, loads = verify(path, profile == "legacy", heap_enabled, task_context)
    check_arm_profile(path.read_bytes(), sections.get(".ARM.attributes"))
    ram_start = symbols["__sram_start"]
    ram_end = symbols["__sram_end"]
    if task_context:
        require(symbols["__text_start"] < symbols["__text_end"], "Empty code range")
    require((ram_start, ram_end) == (0x20000000, 0x20010000), "Unexpected SRAM range")

    specs = (
        (".data", "__data_start", "__data_end", 4, None, 1),
        (".bss", "__bss_start", "__bss_end", 4, None, 8),
        (".fault_stack", "__fault_stack_bottom", "__fault_stack_top", 8,
         0 if profile == "legacy" else 4096, 8),
        (".memory_pool", "__pool_start", "__pool_end", 32,
         2048 if profile in ("no-heap", "tlsf") else 0, 8),
        (".memory_heap", "__heap_start", "__heap_end", 8, expected_heap, 8),
        (".stack", "__stack_bottom", "__stack_top", 8, 4096, 8),
    )
    previous = ram_start
    used = 0
    unused = []
    allowed = set()
    for name, start_name, end_name, align, expected_size, kind in specs:
        require(start_name in symbols and end_name in symbols, f"{name}: missing symbols")
        start, end = symbols[start_name], symbols[end_name]
        size = end - start
        require(previous <= start <= end <= ram_end, f"{name}: overlaps or exceeds SRAM")
        require(start % align == 0 and end % align == 0, f"{name}: unaligned range")
        require(expected_size is None or size == expected_size, f"{name}: unexpected size")
        header = sections.get(name)
        if size:
            require(header is not None and header[1] == kind and header[2] & 2
                    and header[3] == start and header[5] == size, f"{name}: bad ELF section")
            allowed.add(name)
        elif header is not None:
            require(header[5] == 0, f"{name}: expected an empty section")
        if start > previous:
            unused.append((previous, start))
        previous = end
        used += size
        print(f"  {name:12} [{start:#010x}, {end:#010x}) {size:5} bytes")
    require(previous == ram_end, "MSP does not end at top of SRAM")
    for name, h in sections.items():
        if h[2] & 2 and h[5] and ram_start <= h[3] < ram_end:
            require(name in allowed, f"Unexpected allocated SRAM section {name}")
    data_start, data_end, data_load = (symbols[n] for n in ("__data_start", "__data_end", "__data_load"))
    require(0x08000000 <= data_load <= data_load + data_end - data_start <= 0x08080000,
            "Invalid .data load address")
    if data_start != data_end:
        require(any(virtual == data_start and physical == data_load and size == data_end - data_start
                    for virtual, physical, size, _ in loads), "Missing .data load segment")
    for virtual, _, size, memory_size in loads:
        if ram_start <= virtual < ram_end:
            require(virtual + memory_size <= ram_end and (size == 0 or virtual == data_start),
                    "Unexpected file-backed or out-of-bounds SRAM segment")
    require(used + sum(end - start for start, end in unused) == ram_end - ram_start,
            "SRAM accounting mismatch")
    flash_end = max(start + size for start, size in flash_segments)
    print(f"  Flash span   [0x08000000, {flash_end:#010x})")
    print(f"  SRAM used    {used} bytes; unallocated {ram_end - ram_start - used} bytes")
    for start, end in unused:
        print(f"  SRAM gap     [{start:#010x}, {end:#010x}) {end - start} bytes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("--profile", required=True, choices=("legacy", "m2", "no-heap", "tlsf"))
    parser.add_argument("--heap-bytes", type=int, help="Expected configured TLSF arena size")
    args = parser.parse_args()
    if args.heap_bytes is not None and args.profile != "tlsf":
        parser.error("--heap-bytes requires --profile tlsf")
    check(args.elf, args.profile, (args.heap_bytes if args.heap_bytes is not None else 12288)
          if args.profile == "tlsf" else 0)
