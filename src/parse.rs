//! yieldx 的离线解析器：只解析**显式标注为合成样本**的 JSON 夹具。
//!
//! 本层 `access_mode = none`，没有任何 live 来源；所有输入都是磁盘夹具，
//! 因此解析入口**要求**顶层 `"_synthetic": true`：未标注来源的 JSON MUST 被拒绝，
//! 以免把来源不明的数据当成源事实。
//!
//! **原子失败**：未知字段、缺字段、非法令牌一律拒绝（MUST NOT 静默忽略）。
//! **重复身份**：本层选择**拒绝**（不静默去重），由
//! [`YieldCurveBatch::new`] 的批校验承担。

use serde::Deserialize;

use crate::error::{YieldCurveError, YieldCurveResult};
use crate::value::{
    Date, Frequency, Unit, YieldCurveBatch, YieldCurveCode, YieldCurveConvention, YieldCurveKind,
    YieldCurveMissingReason, YieldCurvePoint, YieldCurvePointIdentity, YieldCurvePointOrigin,
    YieldCurveRate, YieldCurveTenor,
};

/// 接受的夹具文档类型。
const DOCUMENT_KIND: &str = "kernel_fixture";

/// 夹具文档。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureDocument {
    _synthetic: bool,
    #[serde(default)]
    _note: Option<String>,
    kind: String,
    frequency: String,
    unit: String,
    points: Vec<FixturePoint>,
}

/// 夹具中的一条曲线点。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixturePoint {
    source: String,
    series: String,
    currency: String,
    valuation_date: String,
    maturity: String,
    curve_kind: String,
    #[serde(default)]
    vintage: Option<String>,
    #[serde(default)]
    rate_percent: Option<f64>,
    #[serde(default)]
    missing_reason: Option<String>,
    origin: String,
    convention: String,
    #[serde(default)]
    derived_from: Vec<FixtureIdentity>,
}

/// 夹具中的一条派生输入身份。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureIdentity {
    source: String,
    series: String,
    currency: String,
    valuation_date: String,
    maturity: String,
    curve_kind: String,
    #[serde(default)]
    vintage: Option<String>,
}

/// 解析一份曲线批夹具（JSON）。
///
/// # Errors
///
/// - JSON 结构不符（未知字段 / 缺字段 / 类型不符）→ [`YieldCurveError::Invalid`]
/// - 未标注 `_synthetic = true`、文档类型不符、令牌未知、取值歧义
///   → [`YieldCurveError::SemanticallyRejected`]
/// - 取值既非数值也未给具名原因 → [`YieldCurveError::Missing`]
/// - 批内重复身份 / 派生点不完整 → [`YieldCurveError::SemanticallyRejected`]
pub fn parse_yield_curve_batch(input: &str) -> YieldCurveResult<YieldCurveBatch> {
    let document: FixtureDocument = serde_json::from_str(input).map_err(json_error)?;
    if !document._synthetic {
        return Err(YieldCurveError::SemanticallyRejected(
            "夹具必须显式标注 \"_synthetic\": true；未标注来源的输入不得当作源事实".into(),
        ));
    }
    if document.kind != DOCUMENT_KIND {
        return Err(YieldCurveError::SemanticallyRejected(format!(
            "文档类型须为 {DOCUMENT_KIND}（实际长度 {}）",
            document.kind.len()
        )));
    }
    let frequency = parse_frequency(&document.frequency)?;
    let unit = parse_unit(&document.unit)?;
    let mut points = Vec::with_capacity(document.points.len());
    for point in &document.points {
        points.push(build_point(point)?);
    }
    YieldCurveBatch::new(points, frequency, unit)
}

/// 构造一条曲线点。
fn build_point(point: &FixturePoint) -> YieldCurveResult<YieldCurvePoint> {
    let identity = build_identity(
        &point.source,
        &point.series,
        &point.currency,
        &point.valuation_date,
        &point.maturity,
        &point.curve_kind,
        point.vintage.as_deref(),
    )?;
    let rate = match (point.rate_percent, point.missing_reason.as_deref()) {
        (Some(_), Some(_)) => {
            return Err(YieldCurveError::SemanticallyRejected(
                "rate_percent 与 missing_reason 不得同时给出（取值歧义）".into(),
            ));
        }
        (None, None) => {
            return Err(YieldCurveError::Missing(
                "rate_percent 与 missing_reason 必须恰有其一".into(),
            ));
        }
        (Some(value), None) => YieldCurveRate::Present(value),
        (None, Some(reason)) => YieldCurveRate::Missing(parse_missing_reason(reason)?),
    };
    let origin = match point.origin.as_str() {
        "official" => YieldCurvePointOrigin::Official,
        "derived" => YieldCurvePointOrigin::Derived,
        other => {
            return Err(YieldCurveError::SemanticallyRejected(format!(
                "未知的 origin 令牌（长度 {}）",
                other.len()
            )));
        }
    };
    let derived_from = if point.derived_from.is_empty() {
        None
    } else {
        let mut inputs = Vec::with_capacity(point.derived_from.len());
        for input in &point.derived_from {
            inputs.push(build_identity(
                &input.source,
                &input.series,
                &input.currency,
                &input.valuation_date,
                &input.maturity,
                &input.curve_kind,
                input.vintage.as_deref(),
            )?);
        }
        Some(inputs)
    };
    YieldCurvePoint::new(
        identity,
        rate,
        origin,
        YieldCurveConvention::new(&point.convention)?,
        derived_from,
    )
}

/// 构造一条身份七元组。
fn build_identity(
    source: &str,
    series: &str,
    currency: &str,
    valuation_date: &str,
    maturity: &str,
    curve_kind: &str,
    vintage: Option<&str>,
) -> YieldCurveResult<YieldCurvePointIdentity> {
    Ok(YieldCurvePointIdentity {
        source: YieldCurveCode::new(source)?,
        series: YieldCurveCode::new(series)?,
        currency: YieldCurveCode::new(currency)?,
        valuation_date: Date::parse(valuation_date)?,
        maturity: YieldCurveTenor::parse(maturity)?,
        curve_kind: YieldCurveKind::parse(curve_kind)?,
        vintage: match vintage {
            Some(value) => Some(YieldCurveCode::new(value)?),
            None => None,
        },
    })
}

/// 解析频率令牌。
fn parse_frequency(token: &str) -> YieldCurveResult<Frequency> {
    match token {
        "daily" => Ok(Frequency::Daily),
        "weekly" => Ok(Frequency::Weekly),
        "monthly" => Ok(Frequency::Monthly),
        "quarterly" => Ok(Frequency::Quarterly),
        "annual" => Ok(Frequency::Annual),
        "event" => Ok(Frequency::Event),
        "irregular" => Ok(Frequency::Irregular),
        other => Err(YieldCurveError::SemanticallyRejected(format!(
            "未知的频率令牌（长度 {}）",
            other.len()
        ))),
    }
}

/// 解析单位令牌。
fn parse_unit(token: &str) -> YieldCurveResult<Unit> {
    match token {
        "percent" => Ok(Unit::Percent),
        "bp" => Ok(Unit::BasisPoint),
        "undeclared" => Ok(Unit::Undeclared),
        other => Err(YieldCurveError::SemanticallyRejected(format!(
            "未知的单位令牌（长度 {}）；不得静默假定单位",
            other.len()
        ))),
    }
}

/// 解析缺失原因令牌。
fn parse_missing_reason(token: &str) -> YieldCurveResult<YieldCurveMissingReason> {
    match token {
        "source_blank" => Ok(YieldCurveMissingReason::SourceBlank),
        "not_published_yet" => Ok(YieldCurveMissingReason::NotPublishedYet),
        "not_applicable" => Ok(YieldCurveMissingReason::NotApplicable),
        other => Err(YieldCurveError::SemanticallyRejected(format!(
            "未知的 missing_reason 令牌（长度 {}）",
            other.len()
        ))),
    }
}

/// 把 JSON 解析错误映射为本层错误（**不回显**原始内容）。
fn json_error(error: serde_json::Error) -> YieldCurveError {
    YieldCurveError::Invalid(format!(
        "JSON 结构不符（行 {}，列 {}）",
        error.line(),
        error.column()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{
      "_synthetic": true,
      "_note": "合成样本",
      "kind": "kernel_fixture",
      "frequency": "daily",
      "unit": "percent",
      "points": [
        {
          "source": "treasury", "series": "DS06", "currency": "USD",
          "valuation_date": "2026-08-14", "maturity": "10Y",
          "curve_kind": "nominal", "rate_percent": 4.25,
          "origin": "official", "convention": "act365f"
        },
        {
          "source": "treasury", "series": "DS06", "currency": "USD",
          "valuation_date": "2026-08-14", "maturity": "5Y",
          "curve_kind": "nominal", "missing_reason": "source_blank",
          "origin": "official", "convention": "act365f"
        }
      ]
    }"#;

    #[test]
    fn synthetic_document_parses() {
        let batch = parse_yield_curve_batch(OK).expect("解析成功");
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.unit, Unit::Percent);
        assert_eq!(batch.frequency, Frequency::Daily);
        assert_eq!(batch.points()[0].rate, YieldCurveRate::Present(4.25));
        assert_eq!(
            batch.points()[1].rate,
            YieldCurveRate::Missing(YieldCurveMissingReason::SourceBlank)
        );
    }

    #[test]
    fn unmarked_or_wrong_kind_documents_are_rejected() {
        let unmarked = OK.replace("\"_synthetic\": true", "\"_synthetic\": false");
        assert_eq!(
            parse_yield_curve_batch(&unmarked)
                .expect_err("未标注合成必须拒绝")
                .kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
        let wrong_kind = OK.replace("\"kernel_fixture\"", "\"live_dump\"");
        assert!(parse_yield_curve_batch(&wrong_kind).is_err());
    }

    #[test]
    fn unknown_and_missing_fields_are_rejected_atomically() {
        let unknown = OK.replace(
            "\"unit\": \"percent\",",
            "\"unit\": \"percent\", \"extra\": 1,",
        );
        assert!(
            parse_yield_curve_batch(&unknown).is_err(),
            "未知字段必须拒绝"
        );

        let missing = OK.replace("\"maturity\": \"10Y\",", "");
        assert!(parse_yield_curve_batch(&missing).is_err(), "缺字段必须拒绝");
    }

    #[test]
    fn ambiguous_and_absent_rate_are_rejected() {
        let ambiguous = OK.replace(
            "\"rate_percent\": 4.25,",
            "\"rate_percent\": 4.25, \"missing_reason\": \"source_blank\",",
        );
        assert_eq!(
            parse_yield_curve_batch(&ambiguous)
                .expect_err("歧义必须拒绝")
                .kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );

        let absent = OK.replace("\"rate_percent\": 4.25,", "");
        assert_eq!(
            parse_yield_curve_batch(&absent)
                .expect_err("缺取值必须拒绝")
                .kind(),
            crate::YieldCurveErrorKind::Missing
        );
    }

    #[test]
    fn bad_tokens_and_dates_are_rejected() {
        assert!(
            parse_yield_curve_batch(&OK.replace("\"10Y\"", "\"10y\"")).is_err(),
            "期限大小写必须敏感"
        );
        assert!(
            parse_yield_curve_batch(&OK.replace("\"2026-08-14\"", "\"2026-8-14\"")).is_err(),
            "日期必须严格 ISO"
        );
        assert!(
            parse_yield_curve_batch(&OK.replace("\"daily\"", "\"hourly\"")).is_err(),
            "未知频率必须拒绝"
        );
        assert!(
            parse_yield_curve_batch(&OK.replace("\"percent\"", "\"parsec\"")).is_err(),
            "未知单位必须拒绝"
        );
        assert!(
            parse_yield_curve_batch(&OK.replace("\"official\"", "\"estimated\"")).is_err(),
            "未知 origin 必须拒绝"
        );
    }

    #[test]
    fn duplicate_identity_in_fixture_is_rejected() {
        let first = r#"{"source":"treasury","series":"DS06","currency":"USD","valuation_date":"2026-08-14","maturity":"10Y","curve_kind":"nominal","rate_percent":4.25,"origin":"official","convention":"act365f"}"#;
        let second = r#"{"source":"treasury","series":"DS06","currency":"USD","valuation_date":"2026-08-14","maturity":"10Y","curve_kind":"nominal","rate_percent":4.30,"origin":"official","convention":"act365f"}"#;
        let duplicate = format!(
            r#"{{"_synthetic":true,"kind":"kernel_fixture","frequency":"daily","unit":"percent","points":[{first},{second}]}}"#
        );
        assert_eq!(
            parse_yield_curve_batch(&duplicate)
                .expect_err("批内重复身份必须拒绝")
                .kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn incomplete_derived_point_in_fixture_is_rejected() {
        let incomplete = r#"{
          "_synthetic": true, "kind": "kernel_fixture",
          "frequency": "daily", "unit": "percent",
          "points": [{
            "source": "treasury", "series": "DS06", "currency": "USD",
            "valuation_date": "2026-08-14", "maturity": "5Y",
            "curve_kind": "nominal", "rate_percent": 4.0,
            "origin": "derived", "convention": "act365f", "derived_from": []
          }]
        }"#;
        let error = parse_yield_curve_batch(incomplete).expect_err("不完整派生必须拒绝");
        assert_eq!(
            error.kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn complete_derived_point_with_vintage_parses() {
        let complete = r#"{
          "_synthetic": true, "kind": "kernel_fixture",
          "frequency": "daily", "unit": "percent",
          "points": [{
            "source": "treasury", "series": "DS06", "currency": "USD",
            "valuation_date": "2026-08-14", "maturity": "5Y",
            "curve_kind": "nominal", "rate_percent": 4.0, "vintage": "v2",
            "origin": "derived", "convention": "act365f",
            "derived_from": [{
              "source": "treasury", "series": "DS06", "currency": "USD",
              "valuation_date": "2026-08-14", "maturity": "2Y",
              "curve_kind": "nominal"
            }]
          }]
        }"#;
        let batch = parse_yield_curve_batch(complete).expect("完整派生点");
        assert_eq!(batch.points()[0].origin, YieldCurvePointOrigin::Derived);
        let key = batch.points()[0].identity.canonical_key();
        assert!(key.contains("vintage=v2"), "vintage 必须进入规范键：{key}");
    }
}
