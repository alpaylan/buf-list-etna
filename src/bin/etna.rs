// ETNA workload runner for buf-list.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: ReadExactPosOnEof | All
//
// Each invocation emits a single JSON line on stdout and exits 0
// (usage errors exit 2).

use buf_list::etna::{property_read_exact_pos_on_eof, PropertyResult};
use crabcheck::quickcheck as crabcheck_qc;
use hegel::{generators as hgen, Hegel, Settings as HegelSettings};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRunner};
use quickcheck::{QuickCheck, ResultStatus, TestResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

const ALL_PROPERTIES: &[&str] = &["ReadExactPosOnEof"];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ───────────── etna tool: replays frozen witness inputs. ─────────────
fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "ReadExactPosOnEof" => {
            // Two frozen witnesses; ensure both pass.
            let case1 = property_read_exact_pos_on_eof(
                vec![b"abc".to_vec(), b"de".to_vec()],
                0,
                5,
            );
            let case2 = property_read_exact_pos_on_eof(
                vec![b"hello".to_vec(), b"xyz".to_vec()],
                3,
                12,
            );
            match (case1, case2) {
                (PropertyResult::Fail(m), _) | (_, PropertyResult::Fail(m)) => Err(m),
                _ => Ok(()),
            }
        }
        _ => {
            return (
                Err(format!("Unknown property: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    (result, Metrics { inputs: 1, elapsed_us })
}

// ───────────── proptest ─────────────
fn chunks_strategy() -> BoxedStrategy<Vec<Vec<u8>>> {
    prop::collection::vec(prop::collection::vec(any::<u8>(), 0..16), 0..8).boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let mut runner = TestRunner::new(ProptestConfig::default());
    let result: Result<(), String> = match property {
        "ReadExactPosOnEof" => {
            let c = counter.clone();
            runner
                .run(
                    &(chunks_strategy(), any::<u32>(), any::<u32>()),
                    move |(chunks, start_pos, extra)| {
                        c.fetch_add(1, Ordering::Relaxed);
                        match property_read_exact_pos_on_eof(chunks, start_pos, extra) {
                            PropertyResult::Pass | PropertyResult::Discard => Ok(()),
                            PropertyResult::Fail(m) => Err(TestCaseError::fail(m)),
                        }
                    },
                )
                .map_err(|e| e.to_string())
        }
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ───────────── quickcheck (fork with `etna` feature) ─────────────
//
// The etna feature on QuickCheck's Testable impl requires `Display` on every
// argument, which `Vec<Vec<u8>>` does not implement. Take scalar seeds and
// expand them deterministically — same trick as the other workloads in this
// benchmark.
static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn seed_to_chunks(seed: u64) -> Vec<Vec<u8>> {
    let n = ((seed >> 60) as usize) % 5 + 1;
    let mut out = Vec::with_capacity(n);
    let mut s = seed;
    for _ in 0..n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let clen = ((s >> 58) as usize) % 8 + 1;
        let mut chunk = Vec::with_capacity(clen);
        for _ in 0..clen {
            s = s
                .wrapping_mul(2862933555777941757)
                .wrapping_add(3037000493);
            chunk.push((s >> 33) as u8);
        }
        out.push(chunk);
    }
    out
}

fn qc_read_exact_pos_on_eof(seed: u64, start: u32, extra: u32) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_read_exact_pos_on_eof(seed_to_chunks(seed), start, extra) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let mut qc = QuickCheck::new().tests(200).max_tests(2000);
    let result = match property {
        "ReadExactPosOnEof" => qc.quicktest(
            qc_read_exact_pos_on_eof as fn(u64, u32, u32) -> TestResult,
        ),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!(
            "quickcheck counterexample: ({})",
            arguments.join(" ")
        )),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".into()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    (status, metrics)
}

// ───────────── crabcheck ─────────────
static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn usize_to_u8(x: usize) -> u8 {
    // Same multiplicative-hash trick as rust-csv to spread small usizes
    // (crabcheck's Arbitrary<usize> tends to stay in 0..~14) across the
    // full byte range.
    let h = (x as u32).wrapping_mul(2654435761);
    (h >> 24) as u8
}

fn nested_usize_to_u8(v: Vec<Vec<usize>>) -> Vec<Vec<u8>> {
    v.into_iter()
        .map(|c| c.into_iter().map(usize_to_u8).collect())
        .collect()
}

fn cc_read_exact_pos_on_eof(
    (chunks, start, extra): (Vec<Vec<usize>>, usize, usize),
) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_read_exact_pos_on_eof(
        nested_usize_to_u8(chunks),
        start as u32,
        extra as u32,
    ) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let result = match property {
        "ReadExactPosOnEof" => crabcheck_qc::quickcheck(cc_read_exact_pos_on_eof),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => Err(format!(
            "crabcheck counterexample: ({})",
            arguments.join(" ")
        )),
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".into()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, metrics)
}

// ───────────── hegel (hegeltest 0.3.7) ─────────────
static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new().test_cases(200).seed(Some(0xBF10_51EE))
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "ReadExactPosOnEof" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let chunks: Vec<Vec<u8>> = tc.draw(
                    hgen::vecs(hgen::vecs(hgen::integers::<u8>()).max_size(12)).max_size(6),
                );
                let start: u32 = tc.draw(hgen::integers::<u32>());
                let extra: u32 = tc.draw(hgen::integers::<u32>());
                if let PropertyResult::Fail(m) =
                    property_read_exact_pos_on_eof(chunks, start, extra)
                {
                    panic!("{m}");
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{property}"),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(format!("hegel found counterexample: {msg}"))
        }
    };
    (status, metrics)
}

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
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

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!("Properties: ReadExactPosOnEof | All");
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    // Silence library-under-test panic noise; frameworks catch panics
    // internally, but the default hook still prints to stderr.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&format!("adapter panic: {msg}")),
            );
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(msg) => emit_json(tool, property, "failed", metrics, Some(&msg), None),
    }
}
