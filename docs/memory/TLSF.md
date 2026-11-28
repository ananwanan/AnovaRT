# TLSF heap

The portable Rust implementation owns one contiguous, exclusively borrowed
arena. AnovaRT uses the two-level bitmap algorithm described by
[Masmano, Ripoll, Crespo and Real (ECRTS 2004)](https://doi.org/10.1109/EMRTS.2004.1311009)
and the published [implementation analysis](https://doi.org/10.1002/spe.858).
The project implements the algorithm in its own Rust code and adds no Cargo
dependency.

## Mapping and selection

| Property | Value |
|---|---:|
| First-level classes | 11 |
| Second-level classes per first level | 8 |
| Second-level index bits | 3 |
| Physical block and base alignment | 8 bytes |
| Minimum physical block | 16 bytes |
| Header and payload offset | 8 bytes |
| Maximum arena | 65,536 bytes including sentinel |
| Default firmware arena | 12,288 bytes |
| Firmware arena configuration | `ANOVART_HEAP_BYTES` |

The linear first class covers sizes below 64 bytes in eight-byte steps.
Each subsequent first class covers an octave, divided into eight bins.
Insertion truncates to the bin containing a free block's actual size.
Lookup first considers the head of the bin containing the normalized request
and checks that block's actual size. Otherwise it rounds the search upward
to a bin's lower bound, masks the current SL bitmap, and uses the next
populated FL when necessary. Trailing-zero selection finds the first set bit.
The Cortex-M3 release disassembly contains CLZ and RBIT/CLZ for the key bit
indices. Eight subdivisions keep the free-list table at 176 bytes using u16
arena offsets. Any future class-width change must account for SL bitmap width,
the linear class boundary and the supported top octave.

A sufficiently large block behind an undersized head in the partially
covered bin can be skipped. Requests at the top of the last octave have the
same limitation when rounding crosses the arena size. An exact-sized head
can be used even at that boundary.

## Physical layout and updates

| Offset within a block | Content |
|---|---|
| 0 | u16 physical size with bit 0 marking free |
| 2 | u16 previous physical block offset, or 0xffff at arena start |
| 4 | u16 requested length for live blocks, zero for free blocks |
| 6 | Reserved header word |
| 8, 10 | u16 previous and next free-list offsets, stored only in free payload |

The next physical block begins at current offset plus physical size. The
zero-sized, used sentinel occupies the final eight bytes and retains the
previous physical offset. A separate 1,024-byte boundary bitmap marks
current physical headers and the sentinel. A deallocation bounds-checks the
address and checks this bitmap before reading a header. The bitmap allows
the integrity scan to reject broken links or a damaged size that would
otherwise cause a read from a payload. List links are maintained by
constant-time insertion/removal. Clearing the last head clears the relevant
SL bit and, when the row becomes empty, the FL bit.

Allocation rejects zero-length layouts and uses checked math to add the
header and align the physical size. For alignment greater than eight, it
searches for the normalized size plus alignment plus eight bytes. This
covers the largest prefix needed to align the payload while keeping any free
prefix at least 16 bytes. A tail shorter than 16 bytes is charged to the
allocation. Conservative padding can cause a failure even when a specific
free block happens to be aligned and could otherwise satisfy the request.
After release, the allocator removes up to two free neighbors, combines
their sizes, updates the succeeding boundary tag and inserts one free block.
It does not retain adjacent free blocks.

## Ownership and timing

The core has no CPU-specific dependencies. It operates through exclusive
mutable access, derives arena addresses from the owned region and returns
raw non-null payload addresses. Callers must respect the allocation length,
own any block they release and cease access before release. A recycled
address cannot be distinguished from the previous owner by address alone.
Ordinary allocation and release trust live allocator metadata. The
diagnostic scan checks recognizable metadata corruption and is not a general
memory-corruption recovery mechanism.

The STM32 global adapter is selected by `heap-tlsf`. It uses M2's PRIMASK
guard, including nested restoration, and rejects Handler Mode before accessing
heap state. `M3_HANDLER_REJECTIONS` provides a SWD-visible counter. Allocation
returns null on failure. An invalid global deallocation or one attempted
from Handler Mode parks the core. Ordinary Rust container OOM uses the stable
`alloc` panic path and the existing bounded terminal writer. Use fallible
container operations where the caller needs recovery. Shared heap guards do
not format logs, and NMI/HardFault handlers never call the heap.

With fixed tables the metadata paths inspect a bounded number of bitmap
words, heads and physical neighbors. Their work is independent of arena
occupancy and list length. Default `GlobalAlloc` realloc can copy up to
the old or new payload length, whichever is smaller, and alloc_zeroed
clears the requested length. Both are linear in the bytes accessed.
Diagnostics traverse the physical chain and free lists under exclusive
access and provide a separate, linear-time integrity and fragmentation
measurement. Firmware runs those scans during boot validation.

## Accounting and measurements

- `used` includes live headers, per-allocation padding and unusably small tails.
  `requested` counts only live caller payload. Difference after subtracting
  live headers is internal padding. Peaks track physical occupied bytes.
- `free()` counts physical free bytes, including the headers of free blocks.
  Diagnostic free payload excludes free headers. The four free-list link bytes
  per free block reside in payload and become usable when allocated.
- `metadata()` counts the controller, all current headers including the
  sentinel and four link bytes per free block. The controller includes
  176 bytes of list heads and the 1,024-byte boundary bitmap. The STM32
  controller occupies 1,256 bytes. With one recovered free block, the
  total logical metadata is 1,276 bytes. These figures
  overlap the SRAM accounting categories shown in the ELF report.
- External fragmentation is `(free_payload - largest_free_payload) / free_payload`.
  It is zero when no free payload remains. The largest physically free payload
  can exceed what a particular size and alignment search can obtain.

The DWT profile emits cycle observations for the pool, TLSF, coalescing,
splitting, high alignment, exhausted lookup and the PRIMASK call path.
It also samples protected calls with SysTick enabled. Measurements are
pending board execution. Observed bounds and compiler output alone are
insufficient to establish WCET. See the [hardware
procedure](../validation/M3-hardware-template.md).
