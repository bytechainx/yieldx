#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 005）：把 `docs/标准.md` 的每个 `##` 章节转成可执行断言。
//!
//! 章节与断言函数须与 `docs/标准.md` 的 `##` 章节 1:1（检查器按标题逐字比对）。
//!
//! // SPEC-MAP: S-1 | 1. Kernel 义务与能力边界 | assert_kernel_boundary
//! // SPEC-MAP: S-2 | 2. 期限与曲线种类标准 | assert_tenor_standard
//! // SPEC-MAP: S-3 | 3. 曲线点身份与规范键标准 | assert_identity_standard
//! // SPEC-MAP: S-4 | 4. 官方与派生二分标准 | assert_origin_standard
//! // SPEC-MAP: S-5 | 5. 曲线批与解析标准 | assert_batch_and_parse_standard
//! // SPEC-MAP: S-6 | 6. 路由接收端与 provider 边界 | assert_routing_standard
//! // SPEC-MAP: S-7 | 7. publication 语义与合成夹具声明 | assert_publication_and_fixture_standard

use yieldx::{
    guard_provider_adapter_scope, is_formal_pit_eligible, kernel_owns, parse_yield_curve_batch,
    receive_routed_batch, validate_curve_batch, validate_curve_point,
    yield_curve_publication_semantics, AvailabilityEvidence, Date, Frequency, Period,
    PitEligibility, TimePrecision, Unit, YieldCurveBatch, YieldCurveCapability, YieldCurveCode,
    YieldCurveConvention, YieldCurveKind, YieldCurvePoint, YieldCurvePointIdentity,
    YieldCurvePointOrigin, YieldCurveRate, YieldCurveRouteSource, YieldCurveTenor,
    PROVIDER_ADAPTERS_IMPLEMENTED, PROVIDER_ADAPTERS_PLANNED, ROUTE_SOURCES, TENOR_CATALOG,
};

/// 运行期递归收集本仓 `src/` 下全部 `.rs`（**不**用手写文件清单，避免新增文件成为扫描盲区）。
///
/// 以 `CARGO_MANIFEST_DIR` 为根，**不依赖 cwd**；目录不可读即失败（不静默跳过）。
fn source_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src 目录可读") {
            let path = entry.expect("目录项可读").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(root.is_dir(), "src 目录必须存在：{}", root.display());
    let mut out = Vec::new();
    walk(&root, &mut out);
    // 结构性守卫：枚举若整体失效，`src/lib.rs` 会缺席 —— 此时 MUST 失败而不是空转通过。
    assert!(
        out.iter().any(|path| path.ends_with("src/lib.rs")),
        "递归枚举未找到 src/lib.rs，扫描面不可信（枚举 {0} 个文件）",
        out.len()
    );
    out
}

fn code(value: &str) -> YieldCurveCode {
    YieldCurveCode::new(value).expect("合法令牌")
}

fn identity(maturity: YieldCurveTenor) -> YieldCurvePointIdentity {
    YieldCurvePointIdentity {
        source: code("us_treasury_yield"),
        series: code("par_yield"),
        currency: code("USD"),
        valuation_date: Date::parse("2026-08-14").expect("合法日期"),
        maturity,
        curve_kind: YieldCurveKind::Nominal,
        vintage: None,
    }
}

fn official(maturity: YieldCurveTenor, rate: f64) -> YieldCurvePoint {
    YieldCurvePoint::new(
        identity(maturity),
        YieldCurveRate::Present(rate),
        YieldCurvePointOrigin::Official,
        YieldCurveConvention::new("act365f").expect("合法惯例"),
        None,
    )
    .expect("合法官方点")
}

/// S-1：kernel 只做 L0 值对象与批校验；不拥有 HTTP / 认证 / 缓存 / 再分发；不批 provider 授权。
#[test]
fn assert_kernel_boundary() {
    for owned in [
        YieldCurveCapability::L0ValueObjects,
        YieldCurveCapability::BatchValidation,
    ] {
        assert!(kernel_owns(owned), "kernel 拥有 {owned:?}");
    }
    for not_owned in [
        YieldCurveCapability::HttpFetch,
        YieldCurveCapability::Authentication,
        YieldCurveCapability::Cache,
        YieldCurveCapability::Redistribution,
    ] {
        assert!(!kernel_owns(not_owned), "kernel 不拥有 {not_owned:?}");
    }
    let error = guard_provider_adapter_scope().expect_err("kernel 不批 provider 授权");
    assert_eq!(
        error.kind(),
        yieldx::YieldCurveErrorKind::WriteAuthorityDenied
    );
    assert_eq!(PROVIDER_ADAPTERS_PLANNED, 32);
    assert_eq!(PROVIDER_ADAPTERS_IMPLEMENTED, 0, "provider 适配为 0/32");

    // 零端点字面量 / 零凭据读取：**运行期递归**枚举 `src/` 下每一个 `.rs`
    // （不依赖 cwd、不用手写清单，故新增文件自动进入覆盖面）。kernel 同样适用本约束。
    // 需要 URL 字面量的负向用例一律放在 `tests/` 下；
    // MUST NOT 用「运行期拼装 scheme」的方式规避本扫描。
    let sources = source_files();
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("源文件可读");
        for forbidden in ["http://", "https://", "env::var", "from_env"] {
            assert!(
                !text.contains(forbidden),
                "{} 含禁用片段 {forbidden}",
                path.display()
            );
        }
    }
}

/// S-2：期限带排序语义、曲线种类只有三态；日期与期间严格 ISO。
#[test]
fn assert_tenor_standard() {
    assert_eq!(TENOR_CATALOG.len(), 11);
    assert!(YieldCurveTenor::M1 < YieldCurveTenor::Y30, "排序按到期递增");
    assert_eq!(YieldCurveTenor::Y10.months(), 120);
    assert!(YieldCurveTenor::parse("10y").is_err(), "标签大小写敏感");
    for (label, kind) in [
        ("nominal", YieldCurveKind::Nominal),
        ("real", YieldCurveKind::Real),
        ("ois", YieldCurveKind::Ois),
    ] {
        assert_eq!(YieldCurveKind::parse(label).expect("三态"), kind);
    }
    assert!(YieldCurveKind::parse("forward").is_err());
    assert!(Date::parse("2026-08-14").is_ok());
    assert!(Date::parse("2026-2-3").is_err());
    assert!(matches!(Period::parse("2026-08"), Ok(Period::Month { .. })));
}

/// S-3：身份七元组齐备，规范键稳定且随任一维度变化。
#[test]
fn assert_identity_standard() {
    let key = identity(YieldCurveTenor::Y10).canonical_key();
    for expected in [
        "source=us_treasury_yield",
        "series=par_yield",
        "currency=USD",
        "valuation_date=2026-08-14",
        "maturity=10Y",
        "curve_kind=nominal",
        "vintage=-",
    ] {
        assert!(key.contains(expected), "规范键缺少 {expected}：{key}");
    }
    let mut other = identity(YieldCurveTenor::Y10);
    other.maturity = YieldCurveTenor::Y30;
    assert_ne!(other.canonical_key(), key, "maturity 必须进入身份");
    assert!(YieldCurveCode::new("").is_err(), "身份令牌不得为空");
}

/// S-4：官方点与派生点二分；不完整派生必须拒绝。
#[test]
fn assert_origin_standard() {
    let official_point = official(YieldCurveTenor::Y10, 4.25);
    assert_eq!(official_point.origin, YieldCurvePointOrigin::Official);
    assert!(validate_curve_point(&official_point).is_ok());

    let official_with_inputs = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y10),
        rate: YieldCurveRate::Present(4.25),
        origin: YieldCurvePointOrigin::Official,
        convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
        derived_from: Some(vec![identity(YieldCurveTenor::Y2)]),
    };
    assert!(
        validate_curve_point(&official_with_inputs).is_err(),
        "官方点不得声明派生输入"
    );

    let derived_missing_inputs = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y5),
        rate: YieldCurveRate::Present(4.0),
        origin: YieldCurvePointOrigin::Derived,
        convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
        derived_from: None,
    };
    let error = validate_curve_point(&derived_missing_inputs).expect_err("不完整派生必须拒绝");
    assert_eq!(
        error.kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );

    let missing_rate = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y5),
        rate: YieldCurveRate::Missing(yieldx::YieldCurveMissingReason::SourceBlank),
        origin: YieldCurvePointOrigin::Derived,
        convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
        derived_from: Some(vec![identity(YieldCurveTenor::Y2)]),
    };
    assert!(
        validate_curve_point(&missing_rate).is_ok(),
        "具名缺失是合法取值"
    );
}

/// S-5：批内 `canonical_key` 唯一；空批合法；夹具解析原子失败且要求合成标注。
#[test]
fn assert_batch_and_parse_standard() {
    let batch = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y2, 3.9),
            official(YieldCurveTenor::Y10, 4.25),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect("合法批");
    assert_eq!(batch.len(), 2);
    assert!(!batch.is_empty());
    assert!(validate_curve_batch(batch.points()).is_ok());

    let duplicate = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y10, 4.25),
            official(YieldCurveTenor::Y10, 4.50),
        ],
        Frequency::Daily,
        Unit::Percent,
    );
    assert!(duplicate.is_err(), "批内重复身份必须拒绝（不得静默去重）");

    let empty = YieldCurveBatch::new(Vec::new(), Frequency::Daily, Unit::Percent).expect("空批");
    assert!(empty.is_empty());

    let parsed = parse_yield_curve_batch(include_str!("fixtures/us_sovereign_par_official.json"))
        .expect("合成夹具可解析");
    assert_eq!(parsed.len(), 3);
    let negative =
        parse_yield_curve_batch(include_str!("fixtures/negative_incomplete_derived.json"));
    assert!(negative.is_err(), "负向样本必须被拒绝");
}

/// S-6：路由接收端声明齐备；接收即做批校验且拒绝消息带来源标签；不拉取。
#[test]
fn assert_routing_standard() {
    assert_eq!(ROUTE_SOURCES.len(), 4);
    assert_eq!(ROUTE_SOURCES[0].label(), "treasuryx:DS06");
    assert_eq!(ROUTE_SOURCES[1].label(), "treasuryx:DS07");
    assert_eq!(ROUTE_SOURCES[2].label(), "ecbx:YC-dataflow");
    assert_eq!(ROUTE_SOURCES[3].label(), "ukx:S07");

    let batch = receive_routed_batch(
        YieldCurveRouteSource::EcbYcDataflow,
        vec![official(YieldCurveTenor::Y10, 4.25)],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect("接收成功");
    assert_eq!(batch.len(), 1);

    let error = receive_routed_batch(
        YieldCurveRouteSource::TreasuryDs07,
        vec![
            official(YieldCurveTenor::Y10, 4.25),
            official(YieldCurveTenor::Y10, 4.5),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect_err("批校验必须生效");
    assert!(error.to_string().contains("treasuryx:DS07"));
}

/// S-7：publication 三元组固定为推断层；夹具为合成样本、不构成证据。
#[test]
fn assert_publication_and_fixture_standard() {
    assert_eq!(
        yield_curve_publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
    assert!(!is_formal_pit_eligible());

    for raw in [
        include_str!("fixtures/us_sovereign_par_official.json"),
        include_str!("fixtures/us_with_derived_spread.json"),
        include_str!("fixtures/negative_incomplete_derived.json"),
    ] {
        let value: serde_json::Value = serde_json::from_str(raw).expect("夹具必须是合法 JSON");
        assert_eq!(value["_synthetic"], serde_json::Value::Bool(true));
        let note = value["_note"].as_str().expect("夹具须含 _note");
        assert!(note.contains("合成样本"), "夹具须显式标注合成：{note}");
        assert!(
            note.contains("不构成任何证据"),
            "夹具不得被当作证据：{note}"
        );
        assert!(
            !note.contains("实测") && !note.contains("核验 PASS"),
            "夹具不得被表述为实测或核验通过"
        );
    }
}
