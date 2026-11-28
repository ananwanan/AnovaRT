# Stack watermark and canary

For M3, Reset paints the 4 KiB MSP range before the first Rust frame. The
bottom eight bytes contain two `0xDEADBEEF` words. The remaining words
contain `0xA5A5A5A5`. The M2 emergency stack remains dedicated to terminal
exceptions and is never painted or scanned by M3.

The Cortex-M3 reader samples MSP and scans with a stackless assembly loop
while PRIMASK masks configurable exceptions. It reads below the sampled MSP
and stops at the first modified word. The portable `StackRegion` validates
8-byte-aligned bounds and SP and derives the remaining and peak estimates
from the first marked address. The guard is excluded from usable capacity.
An invalid SP or changed canary yields zero remaining bytes.

Watermarks estimate the deepest word written. The sampled SP also accounts
for untouched space in the current frame. A previous frame that left words
unchanged, data matching the fill pattern, or a stack overflow that skips
the guard can escape the check. An overflow can corrupt memory before any
diagnostic executes.

`fill_stack` and `StackRegion::inspect` work with exclusively owned test
buffers and future inactive task stacks; they require base and top alignment
of eight bytes. They must be used before taking the stack live or after
stopping its owner. The future PSP consumer must supply its own current SP
and synchronize against task execution. M3 does not switch SP or create tasks.
