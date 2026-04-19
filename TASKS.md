# buf-list — ETNA Tasks

Total tasks: 4

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task.

## Task Index

| Task | Variant | Framework | Property | Witness | Command |
|------|---------|-----------|----------|---------|---------|
| 001  | `read_exact_pos_on_eof_b58396d_1` | proptest    | `property_read_exact_pos_on_eof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start` | `cargo run --release --bin etna -- proptest ReadExactPosOnEof` |
| 002  | `read_exact_pos_on_eof_b58396d_1` | quickcheck  | `property_read_exact_pos_on_eof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start` | `cargo run --release --bin etna -- quickcheck ReadExactPosOnEof` |
| 003  | `read_exact_pos_on_eof_b58396d_1` | crabcheck   | `property_read_exact_pos_on_eof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start` | `cargo run --release --bin etna -- crabcheck ReadExactPosOnEof` |
| 004  | `read_exact_pos_on_eof_b58396d_1` | hegel       | `property_read_exact_pos_on_eof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start` | `cargo run --release --bin etna -- hegel ReadExactPosOnEof` |

## Witness catalog

Each witness is a deterministic concrete test. Base build: passes. Variant-active build: fails.

- `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` — chunks `["abc", "de"]`, start=0, extra=5 → `read_exact` of 11 bytes from position 0 must EOF and leave `position() == 5`.
- `witness_read_exact_pos_on_eof_case_mid_buffer_start` — chunks `["hello", "xyz"]`, start=3, extra=12 → `read_exact` of 18 bytes from position 3 must EOF and leave `position() == 8`.
