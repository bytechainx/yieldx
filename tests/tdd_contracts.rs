#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 005）。
//!
//! 入口集合 = 本 crate 全部公开入口（含 `validate*` / 守卫 / 解析器 / 值对象构造与解析）。
//! 下表每个入口先在 `/tmp` 变异副本上观测应红、再在本树观测绿。
//!
//! // TDD-PROBE: Date::parse | 变异：接受 `2026/08/15` 作为合法分隔符 | 红=date_parse_is_strict_iso | 绿=date_parse_is_strict_iso
//! // TDD-PROBE: Period::parse | 变异：`YYYY-Q5` 也被接受为季度 | 红=period_parse_covers_four_forms | 绿=period_parse_covers_four_forms
//! // TDD-PROBE: YieldCurveCode::new | 变异：允许空串与前后空白 | 红=code_new_rejects_blank_and_control_chars | 绿=code_new_rejects_blank_and_control_chars
//! // TDD-PROBE: YieldCurveCode::as_str | 变异：as_str 恒返回空串 | 红=code_as_str_round_trips | 绿=code_as_str_round_trips
//! // TDD-PROBE: YieldCurveTenor::parse | 变异：大小写不敏感且接受 `4Y` | 红=tenor_parse_rejects_non_canonical_labels | 绿=tenor_parse_rejects_non_canonical_labels
//! // TDD-PROBE: YieldCurveTenor::months | 变异：`Y30` 的到期月数改为 320 | 红=tenor_months_increase_with_order | 绿=tenor_months_increase_with_order
//! // TDD-PROBE: YieldCurveConvention::new | 变异：允许前后空白 | 红=convention_new_rejects_blank_and_whitespace | 绿=convention_new_rejects_blank_and_whitespace
//! // TDD-PROBE: YieldCurveConvention::as_str | 变异：as_str 恒返回空串 | 红=convention_as_str_round_trips | 绿=convention_as_str_round_trips
//! // TDD-PROBE: YieldCurvePointIdentity::canonical_key | 变异：vintage 不进入规范键 | 红=canonical_key_covers_all_seven_dimensions | 绿=canonical_key_covers_all_seven_dimensions
//! // TDD-PROBE: YieldCurvePoint::new | 变异：跳过派生完整性校验 | 红=point_new_enforces_derived_completeness | 绿=point_new_enforces_derived_completeness
//! // TDD-PROBE: YieldCurveBatch::new | 变异：批内重复身份被静默接受 | 红=batch_new_rejects_duplicate_canonical_key | 绿=batch_new_rejects_duplicate_canonical_key
//! // TDD-PROBE: YieldCurveBatch::points | 变异：points 返回空切片 | 红=batch_points_expose_read_only_slice | 绿=batch_points_expose_read_only_slice
//! // TDD-PROBE: YieldCurveBatch::len | 变异：len 恒返回 0 | 红=batch_len_matches_point_count | 绿=batch_len_matches_point_count
//! // TDD-PROBE: YieldCurveBatch::is_empty | 变异：空批被拒绝 | 红=empty_batch_is_valid_and_reports_empty | 绿=empty_batch_is_valid_and_reports_empty
//! // TDD-PROBE: validate_curve_point | 变异：官方点也允许派生输入 | 红=validate_point_rejects_derived_inputs_on_official | 绿=validate_point_rejects_derived_inputs_on_official
//! // TDD-PROBE: validate_curve_batch | 变异：只查首点，忽略后续重复 | 红=validate_batch_rejects_duplicates | 绿=validate_batch_rejects_duplicates
//! // TDD-PROBE: receive_routed_batch | 变异：路由来源标签不进错误消息 | 红=routed_batch_validates_and_labels_source | 绿=routed_batch_validates_and_labels_source
//! // TDD-PROBE: kernel_owns | 变异：kernel 声称拥有 HTTP 能力 | 红=kernel_owns_only_l0_capabilities | 绿=kernel_owns_only_l0_capabilities
//! // TDD-PROBE: guard_provider_adapter_scope | 变异：守卫放行 provider 授权 | 红=provider_adapter_scope_is_denied | 绿=provider_adapter_scope_is_denied
//! // TDD-PROBE: parse_yield_curve_batch | 变异：缺字段被静默补默认值 | 红=fixture_parse_handles_official_and_derived | 绿=fixture_parse_handles_official_and_derived
//! // TDD-PROBE: yield_curve_publication_semantics | 变异：可得性证据层改为 Official | 红=publication_triple_is_date_inferred_not_eligible | 绿=publication_triple_is_date_inferred_not_eligible
//! // TDD-PROBE: is_formal_pit_eligible | 变异：返回 true（静默升格为正式 PIT） | 红=formal_pit_is_never_eligible | 绿=formal_pit_is_never_eligible
//!
//! 夹具全部为合成样本，见 `tests/fixtures/` 的 `_synthetic` 标注与 `docs/标准.md` §7。

use yieldx::{
    guard_provider_adapter_scope, is_formal_pit_eligible, kernel_owns, parse_yield_curve_batch,
    receive_routed_batch, validate_curve_batch, validate_curve_point,
    yield_curve_publication_semantics, AvailabilityEvidence, Date, Frequency, Period,
    PitEligibility, TimePrecision, Unit, YieldCurveBatch, YieldCurveCapability, YieldCurveCode,
    YieldCurveConvention, YieldCurveKind, YieldCurvePoint, YieldCurvePointIdentity,
    YieldCurvePointOrigin, YieldCurveRate, YieldCurveRouteSource, YieldCurveTenor,
};

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

/// `Date::parse`：只接受严格 ISO `YYYY-MM-DD`。
#[test]
fn date_parse_is_strict_iso() {
    assert!(Date::parse("2026-08-14").is_ok());
    for bad in [
        "2026/08/14",
        "2026-8-14",
        "2026-08-14T00:00:00",
        "2026-02-30",
    ] {
        assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
    }
}

/// `Period::parse`：四种期间形态，季度限 1–4。
#[test]
fn period_parse_covers_four_forms() {
    assert!(matches!(Period::parse("2026-08-14"), Ok(Period::Day(_))));
    assert!(matches!(
        Period::parse("2026-08"),
        Ok(Period::Month { month: 8, .. })
    ));
    assert!(matches!(
        Period::parse("2026-Q4"),
        Ok(Period::Quarter { quarter: 4, .. })
    ));
    assert!(matches!(Period::parse("2026"), Ok(Period::Year(2026))));
    assert!(Period::parse("2026-Q5").is_err(), "季度上限 4");
}

/// `YieldCurveCode::new`：空串、空白、控制字符与超长一律拒绝。
#[test]
fn code_new_rejects_blank_and_control_chars() {
    assert!(YieldCurveCode::new("USD").is_ok());
    assert!(YieldCurveCode::new("").is_err());
    assert!(YieldCurveCode::new("  ").is_err());
    assert!(YieldCurveCode::new(" USD").is_err(), "前后空白必须拒绝");
    assert!(YieldCurveCode::new("a\nb").is_err(), "控制字符必须拒绝");
    assert!(
        YieldCurveCode::new(&"x".repeat(65)).is_err(),
        "超长必须拒绝"
    );
}

/// `YieldCurveCode::as_str`：往返一致。
#[test]
fn code_as_str_round_trips() {
    let token = code("par_yield");
    assert_eq!(token.as_str(), "par_yield");
    assert_eq!(token, code("par_yield"), "同值必须相等");
    assert_ne!(token, code("breakeven_spread"), "不同令牌不得静默等值");
}

/// `YieldCurveTenor::parse`：只接受规范标签且大小写敏感。
#[test]
fn tenor_parse_rejects_non_canonical_labels() {
    assert_eq!(
        YieldCurveTenor::parse("10Y").expect("规范标签"),
        YieldCurveTenor::Y10
    );
    assert!(YieldCurveTenor::parse("10y").is_err(), "大小写必须敏感");
    assert!(YieldCurveTenor::parse("4Y").is_err(), "非规范期限必须拒绝");
    assert!(YieldCurveTenor::parse("").is_err());
}

/// `YieldCurveTenor::months`：到期月数随声明顺序严格递增（排序语义的依据）。
#[test]
fn tenor_months_increase_with_order() {
    assert_eq!(YieldCurveTenor::M1.months(), 1);
    assert_eq!(YieldCurveTenor::Y10.months(), 120);
    assert_eq!(YieldCurveTenor::Y30.months(), 360);
    let mut previous = 0;
    for maturity in [
        YieldCurveTenor::M1,
        YieldCurveTenor::M3,
        YieldCurveTenor::M6,
        YieldCurveTenor::Y1,
        YieldCurveTenor::Y30,
    ] {
        assert!(maturity.months() > previous, "月数必须递增");
        previous = maturity.months();
    }
    assert!(YieldCurveTenor::M1 < YieldCurveTenor::Y30);
}

/// `YieldCurveConvention::new`：空串、空白、控制字符与超长一律拒绝。
#[test]
fn convention_new_rejects_blank_and_whitespace() {
    assert!(YieldCurveConvention::new("act365f").is_ok());
    assert!(YieldCurveConvention::new("").is_err());
    assert!(YieldCurveConvention::new("  ").is_err());
    assert!(YieldCurveConvention::new(" act365f").is_err());
    assert!(YieldCurveConvention::new("a\tb").is_err());
    assert!(YieldCurveConvention::new(&"x".repeat(65)).is_err());
}

/// `YieldCurveConvention::as_str`：往返一致。
#[test]
fn convention_as_str_round_trips() {
    let convention = YieldCurveConvention::new("act365f").expect("合法惯例");
    assert_eq!(convention.as_str(), "act365f");
}

/// `YieldCurvePointIdentity::canonical_key`：七个维度全部进入规范键。
#[test]
fn canonical_key_covers_all_seven_dimensions() {
    let base = identity(YieldCurveTenor::Y10).canonical_key();
    assert!(base.contains("source=us_treasury_yield"));
    assert!(base.contains("series=par_yield"));
    assert!(base.contains("currency=USD"));
    assert!(base.contains("valuation_date=2026-08-14"));
    assert!(base.contains("maturity=10Y"));
    assert!(base.contains("curve_kind=nominal"));
    assert!(base.contains("vintage=-"), "缺 vintage 必须渲染为占位符");

    let mut vintaged = identity(YieldCurveTenor::Y10);
    vintaged.vintage = Some(code("v2"));
    assert_ne!(vintaged.canonical_key(), base, "vintage 必须进入规范键");

    let mut other_kind = identity(YieldCurveTenor::Y10);
    other_kind.curve_kind = YieldCurveKind::Real;
    assert_ne!(
        other_kind.canonical_key(),
        base,
        "curve_kind 必须进入规范键"
    );
}

/// `YieldCurvePoint::new`：派生完整性在构造期即被强制。
#[test]
fn point_new_enforces_derived_completeness() {
    let incomplete = YieldCurvePoint::new(
        identity(YieldCurveTenor::Y5),
        YieldCurveRate::Present(4.0),
        YieldCurvePointOrigin::Derived,
        YieldCurveConvention::new("act365f").expect("合法惯例"),
        None,
    );
    assert!(incomplete.is_err(), "派生点缺输入必须拒绝");

    let complete = YieldCurvePoint::new(
        identity(YieldCurveTenor::Y5),
        YieldCurveRate::Present(4.0),
        YieldCurvePointOrigin::Derived,
        YieldCurveConvention::new("act365f").expect("合法惯例"),
        Some(vec![identity(YieldCurveTenor::Y2)]),
    );
    assert!(complete.is_ok(), "完整派生点必须通过");
}

/// `YieldCurveBatch::new`：批内 `canonical_key` 必须唯一。
#[test]
fn batch_new_rejects_duplicate_canonical_key() {
    let error = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y10, 4.25),
            official(YieldCurveTenor::Y10, 4.30),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect_err("重复身份必须拒绝");
    assert_eq!(
        error.kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );

    let ok = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y2, 3.9),
            official(YieldCurveTenor::Y10, 4.25),
        ],
        Frequency::Daily,
        Unit::Percent,
    );
    assert!(ok.is_ok(), "不同期限可共存");
}

/// `YieldCurveBatch::points`：只读切片按插入顺序暴露全部点。
#[test]
fn batch_points_expose_read_only_slice() {
    let batch = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y2, 3.9),
            official(YieldCurveTenor::Y10, 4.25),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect("合法批");
    let points = batch.points();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].identity.maturity, YieldCurveTenor::Y2);
    assert_eq!(points[1].identity.maturity, YieldCurveTenor::Y10);
}

/// `YieldCurveBatch::len`：点数与切片长度一致。
#[test]
fn batch_len_matches_point_count() {
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
    assert_eq!(batch.len(), batch.points().len());
}

/// `YieldCurveBatch::is_empty`：空批合法且报告为空。
#[test]
fn empty_batch_is_valid_and_reports_empty() {
    let empty =
        YieldCurveBatch::new(Vec::new(), Frequency::Daily, Unit::Percent).expect("空批必须合法");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.frequency, Frequency::Daily);
    assert_eq!(empty.unit, Unit::Percent);
}

/// `validate_curve_point`：官方点不得声明派生输入。
#[test]
fn validate_point_rejects_derived_inputs_on_official() {
    let point = YieldCurvePoint {
        identity: identity(YieldCurveTenor::Y10),
        rate: YieldCurveRate::Present(4.25),
        origin: YieldCurvePointOrigin::Official,
        convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
        derived_from: Some(vec![identity(YieldCurveTenor::Y2)]),
    };
    assert_eq!(
        validate_curve_point(&point).expect_err("必须拒绝").kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );
    assert!(validate_curve_point(&official(YieldCurveTenor::Y10, 4.25)).is_ok());
}

/// `validate_curve_batch`：逐点校验并拒绝**任意位置**的重复身份。
#[test]
fn validate_batch_rejects_duplicates() {
    let ok = YieldCurveBatch::new(
        vec![
            official(YieldCurveTenor::Y2, 3.9),
            official(YieldCurveTenor::Y10, 4.25),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect("合法批");
    assert!(validate_curve_batch(ok.points()).is_ok());

    // 候选切片：重复出现在**非首点**，校验必须逐点扫完而不是只看第一条。
    let duplicate = [
        official(YieldCurveTenor::Y2, 3.9),
        official(YieldCurveTenor::Y10, 4.25),
        official(YieldCurveTenor::Y10, 4.50),
    ];
    assert_eq!(
        validate_curve_batch(&duplicate)
            .expect_err("非首点的重复也必须拒绝")
            .kind(),
        yieldx::YieldCurveErrorKind::SemanticallyRejected
    );
    assert!(
        YieldCurveBatch::new(duplicate.to_vec(), Frequency::Daily, Unit::Percent).is_err(),
        "构造入口同样必须拒绝"
    );
}

/// `receive_routed_batch`：声明接收语义、做批校验，并在拒绝消息里带来源标签。
#[test]
fn routed_batch_validates_and_labels_source() {
    let batch = receive_routed_batch(
        YieldCurveRouteSource::TreasuryDs06,
        vec![official(YieldCurveTenor::Y10, 4.25)],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect("接收成功");
    assert_eq!(batch.len(), 1);

    let error = receive_routed_batch(
        YieldCurveRouteSource::UkS07,
        vec![
            official(YieldCurveTenor::Y10, 4.25),
            official(YieldCurveTenor::Y10, 4.5),
        ],
        Frequency::Daily,
        Unit::Percent,
    )
    .expect_err("重复身份必须拒绝");
    assert!(
        error.to_string().contains("ukx:S07"),
        "拒绝消息须带来源：{error}"
    );
}

/// `kernel_owns`：只拥有 L0 值对象与批校验。
#[test]
fn kernel_owns_only_l0_capabilities() {
    assert!(kernel_owns(YieldCurveCapability::L0ValueObjects));
    assert!(kernel_owns(YieldCurveCapability::BatchValidation));
    for capability in [
        YieldCurveCapability::HttpFetch,
        YieldCurveCapability::Authentication,
        YieldCurveCapability::Cache,
        YieldCurveCapability::Redistribution,
    ] {
        assert!(!kernel_owns(capability), "kernel 不拥有 {capability:?}");
    }
}

/// `guard_provider_adapter_scope`：kernel 不得批任何 provider 授权。
#[test]
fn provider_adapter_scope_is_denied() {
    let error = guard_provider_adapter_scope().expect_err("kernel 不批 provider 授权");
    assert_eq!(
        error.kind(),
        yieldx::YieldCurveErrorKind::WriteAuthorityDenied
    );
}

/// `parse_yield_curve_batch`：合成夹具可解析；未知字段与不完整派生必须拒绝。
#[test]
fn fixture_parse_handles_official_and_derived() {
    let official_batch =
        parse_yield_curve_batch(include_str!("fixtures/us_sovereign_par_official.json"))
            .expect("官方样本可解析");
    assert_eq!(official_batch.len(), 3);

    let derived_batch =
        parse_yield_curve_batch(include_str!("fixtures/us_with_derived_spread.json"))
            .expect("派生样本可解析");
    assert_eq!(derived_batch.len(), 3);
    assert_eq!(
        derived_batch.points()[2].origin,
        YieldCurvePointOrigin::Derived
    );

    let negative =
        parse_yield_curve_batch(include_str!("fixtures/negative_incomplete_derived.json"));
    assert!(negative.is_err(), "不完整派生负向样本必须拒绝");

    let unknown_field = include_str!("fixtures/us_sovereign_par_official.json").replace(
        "\"unit\": \"percent\",",
        "\"unit\": \"percent\", \"extra\": 1,",
    );
    assert!(
        parse_yield_curve_batch(&unknown_field).is_err(),
        "未知字段必须拒绝"
    );
}

/// `yield_curve_publication_semantics`：三元组恒为 `Date` + `Inferred` + `NotEligible`。
#[test]
fn publication_triple_is_date_inferred_not_eligible() {
    assert_eq!(
        yield_curve_publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
}

/// `is_formal_pit_eligible`：恒为 `false`。
#[test]
fn formal_pit_is_never_eligible() {
    assert!(!is_formal_pit_eligible());
    assert_eq!(
        yield_curve_publication_semantics().2,
        PitEligibility::NotEligible
    );
}
