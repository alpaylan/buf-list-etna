//! Fault-localization integration tests for buf-list.
//!
//! One `#[test]` per property in src/bin/etna-faultloc.rs's dispatch.

use buf_list::etna::{property_read_exact_pos_on_eof, PropertyResult};
use crabcheck::quickcheck::{Arbitrary, Mutate};
use rand::Rng;
use std::fmt;

#[derive(Clone)]
struct Chunks(Vec<Vec<u8>>);
impl fmt::Debug for Chunks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<R: Rng> Arbitrary<R> for Chunks {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let outer = rng.random_range(0..8u32) as usize;
        let mut out = Vec::with_capacity(outer);
        for _ in 0..outer {
            let inner = rng.random_range(0..16u32) as usize;
            let mut chunk = Vec::with_capacity(inner);
            for _ in 0..inner {
                chunk.push(rng.random_range(0..=u8::MAX));
            }
            out.push(chunk);
        }
        Chunks(out)
    }
}

impl<R: Rng> Mutate<R> for Chunks {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.0.clone();
        match rng.random_range(0u8..4) {
            0 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                if !out[i].is_empty() {
                    let j = rng.random_range(0..out[i].len());
                    let b = rng.random_range(0u32..8);
                    out[i][j] ^= 1u8 << b;
                }
            }
            1 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                if out[i].len() < 16 {
                    out[i].push(rng.random_range(0..=u8::MAX));
                }
            }
            2 if out.len() < 8 => {
                out.push(vec![]);
            }
            _ if !out.is_empty() => {
                out.pop();
            }
            _ => {}
        }
        Chunks(out)
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn property_read_exact_pos_on_eof_test(input: (Chunks, usize, usize)) -> Option<bool> {
    let (Chunks(cs), start, extra) = input;
    to_opt(property_read_exact_pos_on_eof(cs, start as u32, extra as u32))
}

// Manual JSON emitter (we don't depend on serde_json in dev-deps).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_f64(x: f64) -> String {
    if x.is_finite() {
        format!("{}", x)
    } else {
        "null".to_string()
    }
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        format!(
            "{{\"rank\":{},\"file\":{},\"function\":{},\"start_line\":{},\"end_line\":{},\"ochiai\":{},\"delta\":{},\"panic_overlap\":{},\"confidence\":{},\"confidence_rule\":{}}}",
            s.rank,
            json_escape(&s.region.file),
            json_escape(&s.region.function),
            s.region.start_line,
            s.region.end_line,
            json_f64(s.region.suspiciousness.ochiai as f64),
            json_f64(s.region.delta as f64),
            s.panic_overlap,
            json_escape(&format!("{}", s.confidence)),
            json_escape(s.confidence_rule),
        )
    } else {
        "null".to_string()
    };
    let top_5_items: Vec<String> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            format!(
                "{{\"rank\":{},\"file\":{},\"function\":{},\"start_line\":{},\"end_line\":{},\"confidence\":{},\"confidence_rule\":{},\"panic_overlap\":{}}}",
                s.rank,
                json_escape(&s.region.file),
                json_escape(&s.region.function),
                s.region.start_line,
                s.region.end_line,
                json_escape(&format!("{}", s.confidence)),
                json_escape(s.confidence_rule),
                s.panic_overlap,
            )
        })
        .collect();
    let top_5 = format!("[{}]", top_5_items.join(","));
    let diag_items: Vec<String> = r.diagnostics.iter().map(|d| json_escape(d.tag())).collect();
    let diags = format!("[{}]", diag_items.join(","));
    let out = format!(
        "{{\"status\":{},\"passed\":{},\"discarded\":{},\"n_panics\":{},\"n_suspects\":{},\"top\":{},\"top_5\":{},\"diagnostics\":{}}}",
        json_escape(status),
        r.run.passed,
        r.run.discarded,
        r.n_panics,
        r.suspects.len(),
        top,
        top_5,
        diags,
    );
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_read_exact_pos_on_eof() {
    let report =
        crabcheck::quickcheck_with_locate!(property_read_exact_pos_on_eof_test, "buf_list");
    eprintln!("{report}");
    emit_locate_json(&report);
}
