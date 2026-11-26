# Testing and acceptance

Layers: host unit tests for portable logic; thumbv7m-none-eabi cross-builds; hardware SWD, UART and GPIO verification; logic analyzer for timing when needed. QEMU is supplemental, not a substitute for board verification.

Record commit SHA, date, tool versions, board, command, exit code, expected and actual results, evidence path or URL and tests not run. Store records in docs/validation/Mn-YYYY-MM-DD.md. Each evidence item in progress.json contains commit, environment, result (pass) and artifact. Mark done only when all acceptance points pass. Without hardware, use implemented or blocked rather than fabricated evidence.

Scheduling tests cover preserved registers, stack boundaries, preemption order, concurrency, timeouts, tick wraparound, ISR latency and context-switch timing. Report measurement conditions and maxima; do not claim hard real-time performance from averages. Memory tests cover linker map, stack watermark, OOM, bounds and fragmentation. Driver tests cover errors, timeouts and bounded retries; use backed-up test media for storage.
