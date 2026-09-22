#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! yieldx 热路径：JSON 夹具 → 曲线批（含批内身份唯一性校验）。
//!
//! 只压测本库自有代码（身份构造、规范键计算、唯一性校验、派生完整性校验），
//! 不含任何网络或文件 I/O。`cargo bench` 需配合 `harness = false`。

use std::hint::black_box;
use std::time::Instant;

use yieldx::{parse_yield_curve_batch, YieldCurveBatch};

/// 构造含 `points` 条曲线点的合成夹具文档（字段名与 `docs/标准.md` §3 一致）。
fn synthetic_document(points: usize) -> String {
    let mut body = String::new();
    for index in 0..points {
        if index > 0 {
            body.push(',');
        }
        let year = 2020 + index / 12;
        let month = index % 12 + 1;
        body.push_str(&format!(
            r#"{{"source":"treasury","series":"DS06","currency":"USD","valuation_date":"{year:04}-{month:02}-15","maturity":"10Y","curve_kind":"nominal","rate_percent":4.25,"origin":"official","convention":"act365f"}}"#
        ));
    }
    format!(
        r#"{{"_synthetic":true,"_note":"合成基准样本","kind":"kernel_fixture","frequency":"daily","unit":"percent","points":[{body}]}}"#
    )
}

fn main() {
    const ITERS: u32 = 200;
    let document = synthetic_document(64);

    let warm: YieldCurveBatch = parse_yield_curve_batch(&document).expect("预热解析");
    black_box(warm.len());

    let start = Instant::now();
    let mut points = 0usize;
    for _ in 0..ITERS {
        let batch = parse_yield_curve_batch(black_box(&document)).expect("解析");
        points = points.wrapping_add(batch.len());
        black_box(&batch);
    }
    let elapsed = start.elapsed();
    println!(
        "bench_yieldx_fixture_parse: points={} iters={ITERS} total={elapsed:?} per_iter={:?} accumulated={}",
        document.matches("\"maturity\":\"10Y\"").count(),
        elapsed / ITERS,
        black_box(points)
    );
}
