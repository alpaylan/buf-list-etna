//! Deterministic witness tests for buf-list ETNA variants.
//!
//! Each `witness_<name>_case_<tag>` passes on the base HEAD and fails under the
//! corresponding `etna/<variant>` branch (or with the matching patch applied).
//! Witnesses call `property_<name>` directly with frozen inputs — no
//! proptest/quickcheck/RNG/clock machinery.

use buf_list::etna::{property_read_exact_pos_on_eof, PropertyResult};

fn expect_pass(r: PropertyResult, what: &str) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Fail(m) => panic!("{what}: property failed: {m}"),
        PropertyResult::Discard => panic!("{what}: unexpected discard"),
    }
}

// Variant: read_exact_pos_on_eof_b58396d_1
//
// Multi-chunk BufList ("abc" + "de"), start=0, read_exact(10) must fail with
// UnexpectedEof and leave the cursor at position 5 (== total), matching
// std::io::Cursor<&[u8]>.
#[test]
fn witness_read_exact_pos_on_eof_case_multi_chunk_from_start() {
    expect_pass(
        property_read_exact_pos_on_eof(
            vec![b"abc".to_vec(), b"de".to_vec()],
            0,
            5,
        ),
        "read_exact_pos_on_eof / multi_chunk_from_start",
    );
}

// Variant: read_exact_pos_on_eof_b58396d_1
//
// Starting mid-buffer: total = 8 ("hello" + "xyz"), start=3, read 20 → EOF.
// Buggy code leaves pos at 3; fixed code advances to 8.
#[test]
fn witness_read_exact_pos_on_eof_case_mid_buffer_start() {
    expect_pass(
        property_read_exact_pos_on_eof(
            vec![b"hello".to_vec(), b"xyz".to_vec()],
            3,
            12,
        ),
        "read_exact_pos_on_eof / mid_buffer_start",
    );
}
