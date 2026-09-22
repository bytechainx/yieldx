#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

//! # yieldx —— 收益率曲线 kernel 的 L0 值对象与批校验
//!
//! 把 `specs/adapter/yield_curve.md` 声明的 kernel 义务落成**可编译、可测、可校验**的类型：
//! 期限 / 曲线种类 / 利率惯例 / 曲线点身份 / 官方与派生二分 / 曲线批。
//!
//! ## 能力
//!
//! | 能力 | 状态 |
//! | --- | --- |
//! | L0 值对象（`YieldCurveTenor` / `YieldCurveKind` / `YieldCurveConvention` / `YieldCurvePointIdentity` / `YieldCurvePointOrigin` / `YieldCurveBatch`） | 已实现 |
//! | 批内 `canonical_key` 唯一性校验（重复身份拒绝） | 已实现 |
//! | 派生完整性校验（不完整派生拒绝；官方点不得声明派生输入） | 已实现 |
//! | 曲线路由**接收端**语义声明（treasuryx DS06/DS07、ecbx YC dataflow、ukx S07） | 已实现 |
//! | 离线夹具解析（JSON，要求显式合成标注） | 已实现 |
//! | HTTP 访问 / 认证 / 缓存 / 再分发 | **不拥有**（`kernel_owns` 恒为 `false`） |
//! | 32 个 provider 适配 | **未实现（0/32）**；kernel MUST NOT 据此批任何 provider 授权 |
//! | 曲线构建 / 利差 / 插值 / 拟合 | **不做**（归 analytics） |
//!
//! ## 责任边界
//!
//! - 本库**做**：L0 值对象与身份、批校验、路由接收端的声明语义。
//! - 本库**不做**：拉取、认证、缓存、再分发（`access_mode = none`）。
//! - **本库没有授权面**：`authorization = not_applicable`，故不存在 `authz` 模块，
//!   也不提供任何授权判定函数——kernel 不是 provider，无权批任何 provider 授权。
//! - 零内部耦合：不依赖任何 `bytechainx/*` crate，无 `path` 依赖。
//!
//! ## 非目标
//!
//! - 不为 32 个 provider 建立任何适配（规划目录 ≠ 授权清单）
//! - 不实现曲线构建（bootstrapping / 插值 / 拟合）
//! - 不实现派生指标（利差 / z-score / 期限溢价）
//! - 不拥有存储与分发
//!
//! ## 诚实边界
//!
//! `production_decision = NO-GO`；`implementation_status = skeleton`；
//! 清单 COMPLETE ≠ ship；authorization ≠ Production Ready。
//! 库内夹具全部为**合成样本**，不是真实源数据，不构成任何证据。
//!
//! # 最小示例
//!
//! ```
//! use yieldx::{
//!     validate_curve_batch, Date, Frequency, Unit, YieldCurveBatch, YieldCurveCode,
//!     YieldCurveConvention, YieldCurveKind, YieldCurvePoint, YieldCurvePointIdentity,
//!     YieldCurvePointOrigin, YieldCurveRate, YieldCurveTenor,
//! };
//!
//! let point = YieldCurvePoint::new(
//!     YieldCurvePointIdentity {
//!         source: YieldCurveCode::new("treasury")?,
//!         series: YieldCurveCode::new("DS06")?,
//!         currency: YieldCurveCode::new("USD")?,
//!         valuation_date: Date::parse("2026-08-14")?,
//!         maturity: YieldCurveTenor::Y10,
//!         curve_kind: YieldCurveKind::Nominal,
//!         vintage: None,
//!     },
//!     YieldCurveRate::Present(4.25),
//!     YieldCurvePointOrigin::Official,
//!     YieldCurveConvention::new("act365f")?,
//!     None,
//! )?;
//!
//! let batch = YieldCurveBatch::new(vec![point], Frequency::Daily, Unit::Percent)?;
//! assert_eq!(batch.len(), 1);
//! validate_curve_batch(batch.points())?;
//! # Ok::<(), yieldx::YieldCurveError>(())
//! ```

pub mod error;
pub mod parse;
pub mod pit;
pub mod routing;
pub mod value;

pub use error::{YieldCurveError, YieldCurveErrorKind, YieldCurveResult};
pub use parse::parse_yield_curve_batch;
pub use pit::{
    is_formal_pit_eligible, yield_curve_publication_semantics, AvailabilityEvidence,
    PitEligibility, TimePrecision,
};
pub use routing::{
    guard_provider_adapter_scope, kernel_owns, receive_routed_batch, YieldCurveCapability,
    YieldCurveRouteSource, PROVIDER_ADAPTERS_IMPLEMENTED, PROVIDER_ADAPTERS_PLANNED, ROUTE_SOURCES,
};
pub use value::{
    validate_curve_batch, validate_curve_point, Date, Frequency, Period, Unit, YieldCurveBatch,
    YieldCurveCode, YieldCurveConvention, YieldCurveKind, YieldCurveMissingReason, YieldCurvePoint,
    YieldCurvePointIdentity, YieldCurvePointOrigin, YieldCurveRate, YieldCurveTenor, TENOR_CATALOG,
};
