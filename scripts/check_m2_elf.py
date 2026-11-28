# SPDX-License-Identifier: Apache-2.0
"""Verify STM32F103ZET6 release ELF vectors and report linked resources."""

import argparse
import hashlib
import struct
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def string_at(table, offset):
    return table[offset:table.index(b"\0", offset)].decode()


def verify(path, legacy, heap_enabled=False, task_context=False):
    elf = path.read_bytes()
    require(elf[:7] == b"\x7fELF\x01\x01\x01", "Expected little-endian ELF32")
    (kind, cpu, _, entry, phoff, shoff, _, _, phsize, phnum,
     shsize, shnum, shstr) = struct.unpack_from("<HHIIIIIHHHHHH", elf, 16)
    require((kind, cpu) == (2, 40), "Expected ARM executable")
    headers = [struct.unpack_from("<10I", elf, shoff + n * shsize) for n in range(shnum)]

    def section_data(header):
        return elf[header[4]:header[4] + header[5]]

    names = section_data(headers[shstr])
    sections = {string_at(names, h[0]): h for h in headers}
    symtab = sections[".symtab"]
    strings = section_data(headers[symtab[6]])
    symbols = {}
    for n in range(0, symtab[5], symtab[9]):
        name, value = struct.unpack_from("<II", elf, symtab[4] + n)
        if name:
            symbols[string_at(strings, name)] = value

    vectors = sections[".vectors"]
    text = sections[".text"]
    require(vectors[3] == 0x08000000 and vectors[8] >= 512, "Invalid VTOR address/alignment")
    require(vectors[5] == (16 + 60) * 4, "Invalid vector size")
    words = struct.unpack("<76I", section_data(vectors))
    require(words[0] == symbols["__stack_top"] == 0x20010000, "Invalid initial MSP")
    require(words[1] == symbols["Reset"] == entry, "Reset vector/entry mismatch")

    core_handlers = {
        2: "NmiHandler", 3: "HardFaultHandler", 4: "MemManageHandler",
        5: "BusFaultHandler", 6: "UsageFaultHandler", 11: "m2_svc",
        12: "DebugMonitorHandler", 14: "m2_pendsv", 15: "m2_systick",
    }
    if task_context:
        core_handlers.update({11: "m4_svc", 14: "m4_pendsv"})
    for slot, value in enumerate(words[1:], 1):
        if slot in (7, 8, 9, 10, 13):
            require(value == 0, f"Reserved vector {slot} is nonzero")
            continue
        expected = "Reset" if slot == 1 else "DefaultHandler"
        if not legacy:
            expected = core_handlers.get(slot, expected)
        require(value == symbols[expected], f"Vector {slot} does not point to {expected}")
        require(value & 1 and text[3] <= value - 1 < text[3] + text[5],
                f"Vector {slot} is not a Thumb code address")

    require(symbols["__stack_top"] - symbols["__stack_bottom"] == 4096, "Invalid MSP size")
    emergency = symbols["__fault_stack_top"] - symbols["__fault_stack_bottom"]
    require(emergency == (0 if legacy else 4096), "Invalid emergency MSP size")
    require(symbols["__bss_end"] <= symbols["__fault_stack_bottom"], "BSS overlaps emergency MSP")
    require(symbols["__fault_stack_top"] <= symbols["__stack_bottom"], "MSP overlap")
    require(all(symbols[n] % 8 == 0 for n in ("__stack_bottom", "__fault_stack_top")),
            "Unaligned MSP")
    allocator = any(n.startswith(("__rust_alloc", "__rdl_alloc", "__rg_alloc"))
                    or "__rust_alloc_error_handler" in n
                    or n in ("malloc", "calloc", "realloc", "free") for n in symbols)
    require(allocator == heap_enabled, "Allocator symbols do not match the heap profile")
    if heap_enabled:
        require("M3_HANDLER_REJECTIONS" in symbols, "Missing M3 global allocator diagnostics")

    flash_segments = []
    loads = []
    for n in range(phnum):
        kind, _, virtual, physical, size, memory_size, _, _ = struct.unpack_from("<8I", elf, phoff + n * phsize)
        if kind == 1:
            loads.append((virtual, physical, size, memory_size))
        if kind == 1 and size:
            require(0x08000000 <= physical < physical + size <= 0x08080000,
                    "File-backed load segment exceeds Flash")
            flash_segments.append((physical, size))
    flash = sum(size for _, size in flash_segments)
    span = max(address + size for address, size in flash_segments) - vectors[3]
    static = sections[".data"][5] + sections[".bss"][5]
    print(f"{path}: sha256={hashlib.sha256(elf).hexdigest()}")
    print(f"  entry={entry:#010x} vectors={vectors[3]:#010x}/{vectors[5]} bytes "
          f"alignment={vectors[8]} MSP={words[0]:#010x}")
    print(f"  Flash={flash} bytes loaded, {span} bytes spanned; static SRAM={static} bytes")
    for name in (".vectors", ".text", ".ARM.exidx", ".data", ".bss", ".fault_stack", ".stack"):
        if name in sections:
            h = sections[name]
            print(f"  {name:12} address={h[3]:#010x} bytes={h[5]}")
    if not legacy:
        print("  " + " ".join(f"{name}={symbols[name]:#010x}" for name in
                            (*core_handlers.values(), "DefaultHandler", "m2_fault_entry",
                             "m2_external_dispatch", "M2_FAULT_RECORD")))

    return sections, symbols, flash_segments, loads


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("--legacy", action="store_true", help="Verify an M0 or M1 ELF")
    args = parser.parse_args()
    verify(args.elf, args.legacy)
