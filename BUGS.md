# buf-list — Injected Bugs

Total mutations: 1

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `read_exact_pos_on_eof` | `read_exact_pos_on_eof_b58396d_1` | `patches/read_exact_pos_on_eof_b58396d_1.patch` (applies to `src/cursor/mod.rs`) | `patch` | `b58396d3aadd4c91086c94feb5ef5a29465b7d86` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `read_exact_pos_on_eof_b58396d_1` | `property_read_exact_pos_on_eof` | `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `property_read_exact_pos_on_eof` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. read_exact_pos_on_eof
- **Variant**: `read_exact_pos_on_eof_b58396d_1`
- **Location**: `patches/read_exact_pos_on_eof_b58396d_1.patch` (applies to `src/cursor/mod.rs`)
- **Property**: `property_read_exact_pos_on_eof`
- **Witness(es)**: `witness_read_exact_pos_on_eof_case_multi_chunk_from_start`, `witness_read_exact_pos_on_eof_case_mid_buffer_start`
- **Fix commit**: `b58396d3aadd4c91086c94feb5ef5a29465b7d86` — `try and fix build (#11)`
- **Invariant violated**: After `Cursor::read_exact` fails with `UnexpectedEof` (requested more bytes than remain), the cursor's `position()` must equal the total number of bytes in the `BufList`, matching `std::io::Cursor<&[u8]>::read_exact` on Rust ≥ 1.80 (rust-lang/rust#125404).
- **How the mutation triggers**: The patch removes the `self.set_pos(list, total);` call inside `CursorData::read_exact_impl`, so when the read_exact path errors out the cursor position remains wherever the caller set it rather than advancing to the end of the buffer.
