#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 005）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: 期限排序与到期月数必须一致 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 期限与曲线种类标准 | 结论=保留
//! // AIDD: 规范键对缺失 vintage 渲染占位符而非省略维度 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 曲线点身份与规范键标准 | 结论=保留
//! // AIDD: 官方点声明派生输入必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 官方与派生二分标准 | 结论=保留
//! // AIDD: 派生点输入为空数组（负向样本语义）必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 官方与派生二分标准 | 结论=保留
//! // AIDD: 极大有限取值 1e308 合法、无穷必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 官方与派生二分标准 | 结论=保留
//! // AIDD: 空批合法而含重复身份的批非法 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §5 曲线批与解析标准 | 结论=保留
//! // AIDD: 未标注 _synthetic 的夹具文档必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §5 曲线批与解析标准 | 结论=保留
//! // AIDD: kernel 不得声称拥有 HTTP 等非 L0 能力 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §1 Kernel 义务与能力边界 | 结论=保留

use yieldx::{
    kernel_owns, parse_yield_curve_batch, validate_curve_point, Date, Frequency, Unit,
    YieldCurveBatch, YieldCurveCapability, YieldCurveCode, YieldCurveConvention, YieldCurveKind,
    YieldCurvePoint, YieldCurvePointIdentity, YieldCurvePointOrigin, YieldCurveRate,
    YieldCurveTenor, TENOR_CATALOG,
};

fn identity(maturity: YieldCurveTenor) -> YieldCurvePointIdentity {
    YieldCurvePointIdentity {
        source: YieldCurveCode::new("us_treasury_yield").expect("合法令牌"),
        series: YieldCurveCode::new("par_yield").expect("合法令牌"),
        currency: YieldCurveCode::new("USD").expect("合法令牌"),
        valuation_date: Date::parse("2026-08-14").expect("合法日期"),
        maturity,
        curve_kind: YieldCurveKind::Nominal,
        vintage: None,
    }
}

fn convention() -> YieldCurveConvention {
    YieldCurveConvention::new("act365f").expect("合法惯例")
}

fn official(maturity: YieldCurveTenor, rate: YieldCurveRate) -> YieldCurvePoint {
    YieldCurvePoint::new(
        identity(maturity),
        rate,
        YieldCurvePointOrigin::Official,
        convention(),
        None,
    )
    .expect("合法官方点")
}

/// 边界：声明顺序、`months()` 与 `Ord` 三者必须一致（不得只改其一）。
#[test]
fn tenor_order_and_months_agree() {
    let mut previous_months = 0;
    let mut previous_tenor: Option<YieldCurveTenor> = None;
    for maturity in TENOR_CATALOG {
        assert!(maturity.months() > previous_months, "月数必须递增");
        if let Some(previous) = previous_tenor {
            assert!(previous < maturity, "声明顺序必须与月数一致");
        }
        previous_months = maturity.months();
        previous_tenor = Some(maturity);
    }
}

/// 边界：`vintage` 为 `None` 时规范键仍保留该维度（渲染占位符，不得整段省略）。
#[test]
fn canonical_key_keeps_dimension_when_vintage_absent() {
    let key = identity(YieldCurveTenor::Y10).canonical_key();
    assert!(
        key.ends_with("vintage=-"),
        "缺少占位符会让维度被静默省略：{key}"
    );
    let mut with_vintage = identity(YieldCurveTenor::Y10);
    with_vintage.vintage = Some(YieldCurveCode::new("v2").expect("合法令牌"));
    assert!(with_vintage.canonical_key().ends_with("vintage=v2"));
}

/// 边界：官方点声明派生输入属语义矛盾，必须拒绝。
#[test]
fn official_point_with_inputs_is_rejected() {
    let point = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y10),
        rate: YieldCurveRate::Present(4.25),
        origin: YieldCurvePointOrigin::Official,
        convention: convention(),
        derived_from: Some(vec![identity(YieldCurveTenor::Y2)]),
    };
    assert_eq!(
        validate_curve_point(&point).expect_err("必须拒绝").kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );
}

/// 边界：派生输入为空数组等价于「不完整派生」，必须拒绝。
#[test]
fn derived_point_with_empty_inputs_is_rejected() {
    let point = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y5),
        rate: YieldCurveRate::Present(4.0),
        origin: YieldCurvePointOrigin::Derived,
        convention: convention(),
        derived_from: Some(Vec::new()),
    };
    assert_eq!(
        validate_curve_point(&point).expect_err("必须拒绝").kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );
}

/// 边界：`1e308` 是有限数故合法；溢出为无穷必须拒绝。
#[test]
fn extreme_finite_rate_is_accepted_but_infinity_is_not() {
    assert!(validate_curve_point(&official(
        YieldCurveTenor::Y10,
        YieldCurveRate::Present(1e308)
    ))
    .is_ok());

    // 直接构造（绕过 new 的构造期校验）以验证 validate 自身的无穷检查。
    let infinite = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y10),
        rate: YieldCurveRate::Present(f64::INFINITY),
        origin: YieldCurvePointOrigin::Official,
        convention: convention(),
        derived_from: None,
    };
    assert_eq!(
        validate_curve_point(&infinite)
            .expect_err("无穷必须拒绝")
            .kind(),
        yieldx::YieldCurveErrorKind::Invalid
    );
    // 构造入口同样必须拦住无穷（fail-closed 在两条路径上一致）。
    assert!(YieldCurvePoint::new(
        identity(YieldCurveTenor::Y10),
        YieldCurveRate::Present(f64::INFINITY),
        YieldCurvePointOrigin::Official,
        convention(),
        None,
    )
    .is_err());
}

/// 边界：空批合法；一旦含重复身份即非法（两种极端都不得被静默放过）。
#[test]
fn empty_batch_is_valid_and_duplicate_batch_is_not() {
    assert!(
        YieldCurveBatch::new(Vec::new(), Frequency::Daily, Unit::Percent)
            .expect("空批合法")
            .is_empty()
    );
    assert!(YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y10, YieldCurveRate::Present(4.25)),
            official(YieldCurveTenor::Y10, YieldCurveRate::Present(4.25)),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .is_err());
}

/// 边界：未标注 `_synthetic` 的文档必须拒绝，不得当作源事实。
#[test]
fn unmarked_document_is_rejected() {
    let raw = include_str!("fixtures/us_sovereign_par_official.json")
        .replace("\"_synthetic\": true", "\"_synthetic\": false");
    let error = parse_yield_curve_batch(&raw).expect_err("未标注合成必须拒绝");
    assert_eq!(
        error.kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );
}

/// 边界：kernel 不得声称拥有 HTTP / 缓存等非 L0 能力。
#[test]
fn kernel_never_claims_non_l0_capabilities() {
    for capability in [
        YieldCurveCapability::HttpFetch,
        YieldCurveCapability::Authentication,
        YieldCurveCapability::Cache,
        YieldCurveCapability::Redistribution,
    ] {
        assert!(!kernel_owns(capability), "{capability:?} 不属于 kernel");
    }
    assert!(kernel_owns(YieldCurveCapability::L0ValueObjects));
}
