//! 曲线点与曲线批：取值、值来源二分（官方 / 派生）与**批内身份唯一性**。
//!
//! 本模块承载 kernel 的两条最重要的校验：
//!
//! 1. **批内 `canonical_key` 唯一** —— 重复身份 MUST 被拒绝（不得静默去重、不得后写覆盖）；
//! 2. **派生点必须完整** —— `origin = Derived` 的点 MUST 声明至少一个输入身份，
//!    否则即为清单负向样本 `negative_incomplete_derived.json` 所表达的「不完整派生」，
//!    MUST 被拒绝；反之 `origin = Official` 的点 MUST NOT 声明派生输入。

use std::collections::HashSet;

use crate::error::{YieldCurveError, YieldCurveResult};
use crate::value::{Frequency, Unit, YieldCurveConvention, YieldCurvePointIdentity};

/// 缺失的**具名**原因。MUST NOT 静默把缺失转成 0。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurveMissingReason {
    /// 上游该单元格为空。
    SourceBlank,
    /// 上游尚未发布该期。
    NotPublishedYet,
    /// 该点对该期限不适用。
    NotApplicable,
}

/// 曲线取值。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YieldCurveRate {
    /// 上游给出的数值（f64 原样保留，未做任何换算）。
    Present(f64),
    /// 缺失，并携带具名原因。
    Missing(YieldCurveMissingReason),
}

/// 值来源二分：官方点 vs 派生点。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurvePointOrigin {
    /// 官方点（上游直接给出）。
    Official,
    /// 派生点（由其它曲线点计算得到）。
    Derived,
}

/// 一条曲线点。
#[derive(Debug, Clone, PartialEq)]
pub struct YieldCurvePoint {
    /// 身份七元组。
    pub identity: YieldCurvePointIdentity,
    /// 取值（或其具名缺失）。
    pub rate: YieldCurveRate,
    /// 值来源：官方还是派生。
    pub origin: YieldCurvePointOrigin,
    /// 利率惯例。
    pub convention: YieldCurveConvention,
    /// 派生输入身份：官方点必须为 `None`；派生点必须为 `Some(非空)`。
    pub derived_from: Option<Vec<YieldCurvePointIdentity>>,
}

impl YieldCurvePoint {
    /// 构造并校验一条曲线点。
    ///
    /// # Errors
    ///
    /// 派生输入与 `origin` 不匹配、或取值非有限数时返回
    /// [`YieldCurveError::SemanticallyRejected`] 或 [`YieldCurveError::Invalid`]。
    pub fn new(
        identity: YieldCurvePointIdentity,
        rate: YieldCurveRate,
        origin: YieldCurvePointOrigin,
        convention: YieldCurveConvention,
        derived_from: Option<Vec<YieldCurvePointIdentity>>,
    ) -> YieldCurveResult<Self> {
        let point = Self {
            identity,
            rate,
            origin,
            convention,
            derived_from,
        };
        validate_curve_point(&point)?;
        Ok(point)
    }
}

/// 校验一条曲线点的内部一致性。
///
/// # Errors
///
/// - `origin = Derived` 而派生输入缺失或为空 → [`YieldCurveError::SemanticallyRejected`]
/// - `origin = Official` 却声明了派生输入 → [`YieldCurveError::SemanticallyRejected`]
/// - 取值为 `NaN` 或无穷 → [`YieldCurveError::Invalid`]
pub fn validate_curve_point(point: &YieldCurvePoint) -> YieldCurveResult<()> {
    let date = point.identity.valuation_date;
    crate::value::Date::new(date.year, date.month, date.day)?;
    if let Some(inputs) = &point.derived_from {
        for input in inputs {
            let date = input.valuation_date;
            crate::value::Date::new(date.year, date.month, date.day)?;
        }
    }
    match point.origin {
        YieldCurvePointOrigin::Derived => match &point.derived_from {
            Some(inputs) if !inputs.is_empty() => {}
            _ => {
                return Err(YieldCurveError::SemanticallyRejected(
                    "派生点必须声明至少一个输入身份（不完整派生必须拒绝）".into(),
                ));
            }
        },
        YieldCurvePointOrigin::Official => {
            if point.derived_from.is_some() {
                return Err(YieldCurveError::SemanticallyRejected(
                    "官方点不得声明派生输入".into(),
                ));
            }
        }
    }
    match point.rate {
        YieldCurveRate::Present(value) if !value.is_finite() => Err(YieldCurveError::Invalid(
            "曲线取值不得为 NaN 或无穷（缺失须用具名原因表达）".into(),
        )),
        _ => Ok(()),
    }
}

/// 一批曲线点。
///
/// `points` 私有：唯一入口是 [`YieldCurveBatch::new`]，它保证批内 `canonical_key` 唯一。
#[derive(Debug, Clone, PartialEq)]
pub struct YieldCurveBatch {
    points: Vec<YieldCurvePoint>,
    /// 批的频率。
    pub frequency: Frequency,
    /// 批的源侧单位（保留不换算）。
    pub unit: Unit,
}

impl YieldCurveBatch {
    /// 构造一批曲线点并做批校验。
    ///
    /// 空批是合法的（`len() == 0`）；重复身份 MUST 被拒绝。
    ///
    /// # Errors
    ///
    /// 批内出现重复 `canonical_key`、或任一点自身校验不通过时返回
    /// [`YieldCurveError::SemanticallyRejected`] 等错误。
    pub fn new(
        points: Vec<YieldCurvePoint>,
        frequency: Frequency,
        unit: Unit,
    ) -> YieldCurveResult<Self> {
        validate_curve_batch(&points)?;
        Ok(Self {
            points,
            frequency,
            unit,
        })
    }

    /// 批内全部曲线点（只读）。
    #[must_use]
    pub fn points(&self) -> &[YieldCurvePoint] {
        &self.points
    }

    /// 批内曲线点数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// 批是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// 校验一批曲线点（候选切片，不要求先构造 [`YieldCurveBatch`]）。
///
/// 检查项：逐点自身校验（含派生完整性）+ **批内 `canonical_key` 唯一**。
/// 空切片合法。
///
/// # Errors
///
/// 批内出现重复身份时返回 [`YieldCurveError::SemanticallyRejected`]；
/// 任一点校验失败时原样返回其错误。
pub fn validate_curve_batch(points: &[YieldCurvePoint]) -> YieldCurveResult<()> {
    let mut seen: HashSet<String> = HashSet::new();
    for (index, point) in points.iter().enumerate() {
        validate_curve_point(point)?;
        let key = point.identity.canonical_key();
        if !seen.insert(key) {
            return Err(YieldCurveError::SemanticallyRejected(format!(
                "批内重复身份出现在第 {} 个点；canonical_key 在批内必须唯一（不得静默去重）",
                index + 1
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Date, YieldCurveCode, YieldCurveKind, YieldCurveTenor};

    fn code(value: &str) -> YieldCurveCode {
        YieldCurveCode::new(value).expect("合法令牌")
    }

    fn identity(maturity: YieldCurveTenor) -> YieldCurvePointIdentity {
        YieldCurvePointIdentity {
            source: code("treasury"),
            series: code("DS06"),
            currency: code("USD"),
            valuation_date: Date::new(2026, 8, 14).expect("合法日期"),
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

    fn sample_batch() -> YieldCurveBatch {
        YieldCurveBatch::new(
            vec![official(YieldCurveTenor::Y10, 4.25)],
            Frequency::Daily,
            Unit::Percent,
        )
        .expect("合法批")
    }

    #[test]
    fn official_point_carries_no_derived_inputs() {
        assert!(validate_curve_point(&official(YieldCurveTenor::Y10, 4.25)).is_ok());
        let with_inputs = YieldCurvePoint {
            identity: identity(YieldCurveTenor::Y10),
            rate: YieldCurveRate::Present(4.25),
            origin: YieldCurvePointOrigin::Official,
            convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
            derived_from: Some(vec![identity(YieldCurveTenor::Y2)]),
        };
        let error = validate_curve_point(&with_inputs).expect_err("官方点不得声明派生输入");
        assert_eq!(
            error.kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn incomplete_derived_point_is_rejected() {
        for derived_from in [None, Some(Vec::new())] {
            let point = YieldCurvePoint {
                identity: identity(YieldCurveTenor::Y5),
                rate: YieldCurveRate::Present(4.0),
                origin: YieldCurvePointOrigin::Derived,
                convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
                derived_from,
            };
            let error =
                validate_curve_point(&point).expect_err("不完整派生必须拒绝（负向样本语义）");
            assert_eq!(
                error.kind(),
                crate::YieldCurveErrorKind::SemanticallyRejected
            );
        }
    }

    #[test]
    fn complete_derived_point_is_accepted() {
        let point = YieldCurvePoint::new(
            identity(YieldCurveTenor::Y5),
            YieldCurveRate::Present(4.0),
            YieldCurvePointOrigin::Derived,
            YieldCurveConvention::new("act365f").expect("合法惯例"),
            Some(vec![
                identity(YieldCurveTenor::Y2),
                identity(YieldCurveTenor::Y10),
            ]),
        )
        .expect("完整派生点");
        assert_eq!(point.origin, YieldCurvePointOrigin::Derived);
    }

    #[test]
    fn non_finite_rate_is_rejected() {
        let point = YieldCurvePoint {
            identity: identity(YieldCurveTenor::Y10),
            rate: YieldCurveRate::Present(f64::INFINITY),
            origin: YieldCurvePointOrigin::Official,
            convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
            derived_from: None,
        };
        assert_eq!(
            validate_curve_point(&point)
                .expect_err("无穷必须拒绝")
                .kind(),
            crate::YieldCurveErrorKind::Invalid
        );
        let missing = YieldCurvePoint {
            identity: identity(YieldCurveTenor::Y10),
            rate: YieldCurveRate::Missing(YieldCurveMissingReason::SourceBlank),
            origin: YieldCurvePointOrigin::Official,
            convention: YieldCurveConvention::new("act365f").expect("合法惯例"),
            derived_from: None,
        };
        assert!(validate_curve_point(&missing).is_ok(), "具名缺失是合法取值");
    }

    #[test]
    fn duplicate_canonical_key_is_rejected() {
        let error = YieldCurveBatch::new(
            vec![
                official(YieldCurveTenor::Y10, 4.25),
                official(YieldCurveTenor::Y10, 4.30),
            ],
            Frequency::Daily,
            Unit::Percent,
        )
        .expect_err("批内重复身份必须拒绝");
        assert_eq!(
            error.kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn batch_len_points_and_emptiness() {
        let batch = sample_batch();
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
        assert_eq!(batch.points()[0].identity.maturity, YieldCurveTenor::Y10);
        assert_eq!(batch.frequency, Frequency::Daily);
        assert_eq!(batch.unit, Unit::Percent);

        let empty =
            YieldCurveBatch::new(Vec::new(), Frequency::Daily, Unit::Percent).expect("空批合法");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.points().is_empty());
        assert!(validate_curve_batch(empty.points()).is_ok());
    }

    #[test]
    fn distinct_maturities_share_the_batch() {
        let batch = YieldCurveBatch::new(
            vec![
                official(YieldCurveTenor::Y2, 3.9),
                official(YieldCurveTenor::Y10, 4.25),
                official(YieldCurveTenor::Y30, 4.6),
            ],
            Frequency::Daily,
            Unit::Percent,
        )
        .expect("不同期限可共存");
        assert_eq!(batch.len(), 3);
        assert!(validate_curve_batch(batch.points()).is_ok());
    }

    #[test]
    fn adversarial_public_dates_are_revalidated_in_points_and_inputs() {
        let bad = Date {
            year: 2026,
            month: 2,
            day: 30,
        };
        let mut point = official(YieldCurveTenor::Y10, 1.0);
        point.identity.valuation_date = bad;
        assert!(validate_curve_point(&point).is_err());
        assert!(YieldCurveBatch::new(vec![point], Frequency::Daily, Unit::Percent).is_err());
        let mut input = identity(YieldCurveTenor::Y2);
        input.valuation_date = bad;
        assert!(YieldCurvePoint::new(
            identity(YieldCurveTenor::Y10),
            YieldCurveRate::Present(1.0),
            YieldCurvePointOrigin::Derived,
            YieldCurveConvention::new("SYNTH").unwrap(),
            Some(vec![input])
        )
        .is_err());
    }
}
