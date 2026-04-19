//! ETNA framework-neutral property functions for buf-list.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs and
//! returning `PropertyResult`. Framework adapters (proptest/quickcheck/crabcheck/hegel)
//! in `src/bin/etna.rs` and deterministic witness tests in `tests/etna_witnesses.rs`
//! both call these functions directly — no re-implementation of the invariant
//! inside any adapter.

#![allow(missing_docs)]

use crate::{BufList, Cursor};
use bytes::Bytes;
use std::io::{Cursor as StdCursor, Read};

pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

fn build_buf_list(chunks: &[Vec<u8>]) -> (BufList, Vec<u8>) {
    let mut bl = BufList::new();
    let mut flat: Vec<u8> = Vec::new();
    for c in chunks {
        if c.is_empty() {
            continue;
        }
        flat.extend_from_slice(c);
        bl.push_chunk(Bytes::copy_from_slice(c));
    }
    (bl, flat)
}

// ──────────────────────────────────────────────────────────────────────────
// Property: Cursor::read_exact on EOF advances position to end-of-buffer.
//
// Regression for b58396d3 (buf-list PR #11) — the original `read_exact_impl`
// returned `UnexpectedEof` without updating `self.pos`. On Rust ≥ 1.80 the
// default `io::Read::read_exact` (and `io::Cursor<&[u8]>`) moves the position
// to the end of the available bytes on EOF (rust-lang/rust#125404), so a
// well-behaved `Cursor<&BufList>` must do the same to remain a drop-in
// substitute for the standard cursor.
//
// Invariant: after `read_exact` fails with UnexpectedEof (i.e., requested
// more bytes than remain), the Cursor's `position()` equals the total number
// of bytes in the BufList — matching `std::io::Cursor<&[u8]>::read_exact`.
// ──────────────────────────────────────────────────────────────────────────
pub fn property_read_exact_pos_on_eof(
    chunks: Vec<Vec<u8>>,
    start_pos: u32,
    extra: u32,
) -> PropertyResult {
    let (bl, flat) = build_buf_list(&chunks);
    let total = bl.num_bytes() as u64;

    // Choose an initial position within [0, total]; capping keeps the test
    // domain well-defined.
    let start = (start_pos as u64) % (total + 1);

    // Only test the EOF path: we need buf_len > remaining.
    let remaining = total - start;
    // At least one extra byte beyond remaining, bounded so allocations stay
    // small across randomized inputs.
    let extra_bounded = (extra as u64) % 32 + 1;
    let buf_len = remaining + extra_bounded;
    if buf_len > 4096 {
        return PropertyResult::Discard;
    }

    let mut bl_cursor = Cursor::new(&bl);
    bl_cursor.set_position(start);
    let mut bl_buf = vec![0u8; buf_len as usize];
    let bl_result = std::io::Read::read_exact(&mut bl_cursor, &mut bl_buf);
    let bl_pos_after = bl_cursor.position();

    // Compare against std::io::Cursor<&[u8]> as the behavioral oracle.
    let mut oracle = StdCursor::new(flat.as_slice());
    oracle.set_position(start);
    let mut o_buf = vec![0u8; buf_len as usize];
    let o_result = oracle.read_exact(&mut o_buf);
    let o_pos_after = oracle.position();

    match (bl_result, o_result) {
        (Err(e_bl), Err(e_o)) => {
            if e_bl.kind() != e_o.kind() {
                return PropertyResult::Fail(format!(
                    "error kind mismatch: buf_list={:?} oracle={:?}",
                    e_bl.kind(),
                    e_o.kind()
                ));
            }
        }
        (Ok(()), Ok(())) => {
            // Should not happen given buf_len > remaining, but if somehow both
            // succeed treat it as a discard rather than a mismatch.
            return PropertyResult::Discard;
        }
        (bl_r, o_r) => {
            return PropertyResult::Fail(format!(
                "result shape mismatch: buf_list={:?} oracle={:?}",
                bl_r.is_ok(),
                o_r.is_ok()
            ));
        }
    }

    if bl_pos_after != o_pos_after {
        return PropertyResult::Fail(format!(
            "position mismatch after read_exact EOF: buf_list={} oracle={} (total={} start={} buf_len={})",
            bl_pos_after, o_pos_after, total, start, buf_len
        ));
    }

    // Independently assert the explicit invariant: position must be at total.
    if bl_pos_after != total {
        return PropertyResult::Fail(format!(
            "read_exact EOF did not advance position to total: pos={} total={} (start={} buf_len={})",
            bl_pos_after, total, start, buf_len
        ));
    }

    PropertyResult::Pass
}
