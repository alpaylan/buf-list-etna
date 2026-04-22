# buf-list — ETNA Tasks

Total tasks: 4

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `read_exact_pos_on_eof_b58396d_1` | proptest | `ReadExactPosOnEof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` |
| 002 | `read_exact_pos_on_eof_b58396d_1` | quickcheck | `ReadExactPosOnEof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` |
| 003 | `read_exact_pos_on_eof_b58396d_1` | crabcheck | `ReadExactPosOnEof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` |
| 004 | `read_exact_pos_on_eof_b58396d_1` | hegel | `ReadExactPosOnEof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` |

## Witness Catalog

- `witness_read_exact_pos_on_eof_case_multi_chunk_from_start` — base passes, variant fails
- `witness_read_exact_pos_on_eof_case_mid_buffer_start` — base passes, variant fails
