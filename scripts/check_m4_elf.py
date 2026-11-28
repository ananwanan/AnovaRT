# SPDX-License-Identifier: Apache-2.0
"""Verify M4 sections, vector routing and assembly boundaries."""
import argparse
import re
import subprocess
from pathlib import Path

from check_m2_elf import require
from check_m3_elf import check


def instructions(dump, symbol):
    match = re.search(rf"<{symbol}>:\n(.*?)(?=\n[0-9a-f]+ <|\Z)", dump, re.S)
    require(match is not None, f"Missing disassembly for {symbol}")
    return re.sub(r"\s+", " ", match[1])


def ordered(code, patterns):
    end = 0
    for pattern in patterns:
        match = re.search(pattern, code[end:])
        require(match is not None, f"Missing or out-of-order instruction: {pattern}")
        end += match.end()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("--heap-bytes", type=int, default=0)
    parser.add_argument("--objdump", default="llvm-objdump")
    args = parser.parse_args()
    check(args.elf, "tlsf" if args.heap_bytes else "no-heap", args.heap_bytes, task_context=True)
    dump = subprocess.check_output([
        args.objdump, "-d", "--disassemble-symbols=m4_svc,m4_pendsv,m4_probe_registers",
        str(args.elf),
    ], text=True)
    registers = r"\{r4, r5, r6, r7, r8, r9, r10, r11\}"
    svc = instructions(dump, "m4_svc")
    require(re.match(r"\s*[0-9a-f]+: [0-9a-f ]+ mvn r0, #0x6", svc), "Unexpected SVC entry")
    ordered(svc, [
        r"cmp lr, r0", r"bl [^<]+<m4_first>", r"ldm\.w r0!, " + registers,
        r"msr psp, r0", r"cpsid i", r"msr msp, r1", r"mvn lr, #0x2",
        r"dsb", r"isb", r"cpsie i", r"bx lr",
    ])
    pend = instructions(dump, "m4_pendsv")
    require(not re.search(r"\b(?:push|sub sp)\b", pend.split("mrs r0, psp")[0]),
            "Unexpected PendSV prologue")
    ordered(pend, [
        r"mvn r0, #0x2", r"cmp lr, r0", r"mrs r0, psp",
        r"push \{r0, lr\}", r"bl [^<]+<m4_select>", r"pop\.w \{r1, lr\}",
        r"cbz r0", r"stmdb r1!, " + registers, r"ldm\.w r0!, " + registers,
        r"msr psp, r0", r"isb", r"bx lr",
        r"stmdb r1!, " + registers, r"ldm\.w r0!, " + registers,
        r"msr psp, r0", r"isb", r"bx lr",
    ])
    probe = instructions(dump, "m4_probe_registers")
    ordered(probe, [r"sub sp, #0x4", r"cmp r0, r0", r"str r3, \[r2\]", r"dsb", r"isb", r"add sp, #0x4", r"mrs r0, apsr"])
    print("  M4 SVC, PendSV and register fixture instruction checks passed.")


if __name__ == "__main__":
    main()
