//! 曲线路由的**接收端声明**与 kernel 能力边界。
//!
//! kernel 是曲线点的**接收端**，不是拉取端：上游 provider（`treasuryx` 的 DS06/DS07、
//! `ecbx` 的 YC dataflow、`ukx` 的 S07）把曲线点路由到本域后，本域只做**身份与批校验**；
//! MUST NOT 实现任何拉取、认证、缓存或再分发。
//!
//! 32 个 provider 是**规划目录**（`0/32`），MUST NOT 据此批任何 provider 授权。

use crate::error::{YieldCurveError, YieldCurveResult};
use crate::value::{Frequency, Unit, YieldCurveBatch, YieldCurvePoint};

/// 规划中的 provider 适配数量（规划目录，非授权清单）。
pub const PROVIDER_ADAPTERS_PLANNED: usize = 32;

/// 已实现的 provider 适配数量（当前为 0）。
pub const PROVIDER_ADAPTERS_IMPLEMENTED: usize = 0;

/// 会向本域路由曲线点的上游来源。
///
/// 本枚举只声明**接收语义**，不含任何端点、凭据或限流信息。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurveRouteSource {
    /// 美债日度收益率曲线（1M–30Y par yields）。
    TreasuryDs06,
    /// 美债实际收益率曲线（TIPS）。
    TreasuryDs07,
    /// ECB 的 YC dataflow（点映射须本域批准）。
    EcbYcDataflow,
    /// 英央行/英国 DMO 的 S07 收益率曲线。
    UkS07,
}

/// 已声明的路由来源全集。
pub const ROUTE_SOURCES: [YieldCurveRouteSource; 4] = [
    YieldCurveRouteSource::TreasuryDs06,
    YieldCurveRouteSource::TreasuryDs07,
    YieldCurveRouteSource::EcbYcDataflow,
    YieldCurveRouteSource::UkS07,
];

impl YieldCurveRouteSource {
    /// 来源的可读标签。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::TreasuryDs06 => "treasuryx:DS06",
            Self::TreasuryDs07 => "treasuryx:DS07",
            Self::EcbYcDataflow => "ecbx:YC-dataflow",
            Self::UkS07 => "ukx:S07",
        }
    }
}

/// 接收一批由上游路由来的曲线点：只做身份与批校验，**不拉取、不认证、不缓存**。
///
/// 批校验与 [`YieldCurveBatch::new`] 完全一致（批内 `canonical_key` 唯一、
/// 派生点必须完整）；失败时错误消息带上来源标签以便定位，分类保持不变。
///
/// # Errors
///
/// 批内重复身份、派生点不完整、取值非法时返回对应错误（分类不变）。
pub fn receive_routed_batch(
    source: YieldCurveRouteSource,
    points: Vec<YieldCurvePoint>,
    frequency: Frequency,
    unit: Unit,
) -> YieldCurveResult<YieldCurveBatch> {
    YieldCurveBatch::new(points, frequency, unit).map_err(|error| match error {
        YieldCurveError::SemanticallyRejected(reason) => YieldCurveError::SemanticallyRejected(
            format!("来自 {} 的路由批被拒绝：{reason}", source.label()),
        ),
        other => other,
    })
}

/// kernel 的能力项。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurveCapability {
    /// L0 值对象与身份。
    L0ValueObjects,
    /// 批校验（唯一性与派生完整性）。
    BatchValidation,
    /// HTTP 访问（kernel **不拥有**）。
    HttpFetch,
    /// 认证（kernel **不拥有**）。
    Authentication,
    /// 缓存（kernel **不拥有**）。
    Cache,
    /// 再分发（kernel **不拥有**）。
    Redistribution,
}

/// 本域是否拥有某项能力。
///
/// 只拥有 [`YieldCurveCapability::L0ValueObjects`] 与 [`YieldCurveCapability::BatchValidation`]；
/// HTTP / 认证 / 缓存 / 再分发一律不拥有（属 provider 或运行时层的关切）。
#[must_use]
pub fn kernel_owns(capability: YieldCurveCapability) -> bool {
    matches!(
        capability,
        YieldCurveCapability::L0ValueObjects | YieldCurveCapability::BatchValidation
    )
}

/// provider 授权守卫：kernel **不得**批任何 provider 授权。
///
/// # Errors
///
/// 恒返回 [`YieldCurveError::WriteAuthorityDenied`]。
pub fn guard_provider_adapter_scope() -> YieldCurveResult<()> {
    Err(YieldCurveError::WriteAuthorityDenied(
        "kernel 不是 provider：32 个 provider 为规划目录（0/32），本域不得据此批任何 provider 授权"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{
        Date, YieldCurveCode, YieldCurveConvention, YieldCurveKind, YieldCurvePointIdentity,
        YieldCurvePointOrigin, YieldCurveRate, YieldCurveTenor,
    };

    fn identity(maturity: YieldCurveTenor) -> YieldCurvePointIdentity {
        YieldCurvePointIdentity {
            source: YieldCurveCode::new("treasury").expect("合法令牌"),
            series: YieldCurveCode::new("DS06").expect("合法令牌"),
            currency: YieldCurveCode::new("USD").expect("合法令牌"),
            valuation_date: Date::new(2026, 8, 14).expect("合法日期"),
            maturity,
            curve_kind: YieldCurveKind::Nominal,
            vintage: None,
        }
    }

    fn point(maturity: YieldCurveTenor) -> YieldCurvePoint {
        YieldCurvePoint::new(
            identity(maturity),
            YieldCurveRate::Present(4.25),
            YieldCurvePointOrigin::Official,
            YieldCurveConvention::new("act365f").expect("合法惯例"),
            None,
        )
        .expect("合法点")
    }

    #[test]
    fn all_declared_route_sources_are_distinct_and_labelled() {
        assert_eq!(ROUTE_SOURCES.len(), 4);
        let labels: std::collections::HashSet<&str> =
            ROUTE_SOURCES.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), 4, "来源标签必须互不相同");
        assert!(labels.contains("treasuryx:DS06"));
        assert!(labels.contains("ukx:S07"));
    }

    #[test]
    fn routed_batch_is_accepted_and_duplicates_are_rejected_with_source_label() {
        let batch = receive_routed_batch(
            YieldCurveRouteSource::TreasuryDs06,
            vec![point(YieldCurveTenor::Y2), point(YieldCurveTenor::Y10)],
            Frequency::Daily,
            Unit::Percent,
        )
        .expect("接收成功");
        assert_eq!(batch.len(), 2);

        let error = receive_routed_batch(
            YieldCurveRouteSource::UkS07,
            vec![point(YieldCurveTenor::Y10), point(YieldCurveTenor::Y10)],
            Frequency::Daily,
            Unit::Percent,
        )
        .expect_err("重复身份必须拒绝");
        assert_eq!(
            error.kind(),
            crate::YieldCurveErrorKind::SemanticallyRejected
        );
        assert!(
            error.to_string().contains("ukx:S07"),
            "拒绝消息须带来源标签：{error}"
        );
    }

    #[test]
    fn kernel_owns_only_value_objects_and_batch_validation() {
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

    #[test]
    fn provider_adapters_are_unimplemented_and_unauthorized() {
        assert_eq!(PROVIDER_ADAPTERS_PLANNED, 32);
        assert_eq!(PROVIDER_ADAPTERS_IMPLEMENTED, 0);
        let error = guard_provider_adapter_scope().expect_err("kernel 不得批 provider 授权");
        assert_eq!(
            error.kind(),
            crate::YieldCurveErrorKind::WriteAuthorityDenied
        );
    }
}
