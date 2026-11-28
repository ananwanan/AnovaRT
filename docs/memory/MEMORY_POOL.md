# Fixed-size memory pools

`MemoryPool::new` takes an exclusive caller-owned region of
`MaybeUninit<u8>` aligned to its block size, with a whole number of blocks.
Supported sizes are 16, 32, 64, 128, 256, 512 and 1,024 bytes. Each controller
supports up to 1,024 blocks. M3 uses 64 blocks of 32 bytes and reserves 2 KiB.

Initialization visits each block once to store a u16 free-list link at its
start. Allocation takes the head and sets its bit in the controller's
allocation bitmap. Release validates an address against the arena and the
block boundary, checks the allocation bit and pushes the block to the head.
The bitmap rejects a repeated release while the slot is still free.
Slot addresses can be reused. Pool operations run in O(1) after initialization.

Allocation returns raw uninitialized storage. Treat each returned address as
one owned block, initialize it before reading, and stop using it before
unsafe release. Address/index conversion and allocation are safe operations
because they return raw addresses rather than references. The pool controller
tracks capacity, use, availability and peak use without scanning.

The M3 firmware owns its pool in Thread Mode. A future shared pool on this
single core can use the M2 PRIMASK guard around short metadata operations.
NMI and HardFault must never access a shared pool. M3 leaves ISR resource
policy and task-object lifetime management to subsequent milestones.
