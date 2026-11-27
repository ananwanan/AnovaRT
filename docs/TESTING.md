# Testing and acceptance

Active scope: the Three Blue Whales STM32F103ZET6 board only. Layers: host unit tests for portable logic; thumbv7m-none-eabi cross-builds; hardware SWD, UART and GPIO verification; logic analyzer for timing when needed. QEMU for active-target work, if applicable, is supplemental, not a substitute for board verification.

Record commit SHA, date, tool versions, board, command, exit code, expected and actual results, evidence path or URL and tests not run. Store records in docs/validation/Mn-YYYY-MM-DD.md. Each evidence item in progress.json contains commit, environment, result (pass) and artifact. Mark done only when all acceptance points pass. Without hardware, use implemented or blocked rather than fabricated evidence.

Scheduling tests cover preserved registers, stack boundaries, preemption order, concurrency, timeouts, tick wraparound, ISR latency and context-switch timing. Report measurement conditions and maxima; do not claim hard real-time performance from averages. Memory tests cover linker map, stack watermark, OOM, bounds and fragmentation. Driver tests cover errors, timeouts and bounded retries; use backed-up test media for storage.

## Secondary platforms and release gates

Raspberry Pi 4B and other deferred boards are outside the current verification scope. Do not run secondary cross-builds, QEMU or other simulations, hardware boot, flashing, debugging, peripheral tests, performance tests, or secondary CI matrix jobs. Record their status as deferred / not run, not as pass, fail or blocked for an active milestone. Host tests of portable logic do not establish secondary-board compatibility. No secondary target gates STM32 milestones or releases. M22 is satisfied by the documented portability review and applicable active-target regression evidence, without second-board execution. A future port requires its own explicit activation and validation plan before claiming support.
