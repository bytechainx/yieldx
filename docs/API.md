# yieldx 公开 API

**角色**：收益率曲线 kernel 的 L0 值对象与批校验
**source_id**：`yield_curve` | `kind = kernel` | `access_mode = none` | `authorization = not_applicable`
**production_decision**：`NO-GO`

## 公开消费面

> 本 crate **没有授权面**（kernel 非 provider），故不存在 `authz` 模块，也不导出任何授权类型。

### 错误

| 项 | 一行语义 |
| --- | --- |
| `YieldCurveError` | 本 crate 统一错误（`#[non_exhaustive]`，含 `kind()` / `is_retryable()`） |
| `YieldCurveErrorKind` | 八类反应分类（Invalid / Missing / AuthorizationDenied / RoutedElsewhere / WriteAuthorityDenied / SemanticallyRejected / NotApplicable / Invariant） |
| `YieldCurveResult<T>` | 结果别名 |

### L0 值对象

| 项 | 一行语义 |
| --- | --- |
| `Date::parse` | 严格 ISO `YYYY-MM-DD` 的日期身份（含闰年校验） |
| `Period::parse` | `YYYY` / `YYYY-MM` / `YYYY-Qn` / `YYYY-MM-DD` 四种期间形态 |
| `Frequency` | 观测频率（7 值） |
| `Unit` | 源侧单位（`Percent` / `BasisPoint` / `Undeclared`） |
| `YieldCurveTenor` / `TENOR_CATALOG` | 11 个标准期限；`parse` / `months` / `label`，带排序语义 |
| `YieldCurveKind` | 曲线种类三态（`Nominal` / `Real` / `Ois`）；`parse` / `label` |
| `YieldCurveConvention` | 利率惯例（`new` / `as_str`；取值域由上游声明，本层不虚构全集） |
| `YieldCurveCode` | 上游声明的身份令牌（`new` / `as_str`） |
| `YieldCurvePointIdentity` | 身份七元组 + `canonical_key` |
| `YieldCurvePointOrigin` | 值来源二分：`Official` / `Derived` |
| `YieldCurveRate` / `YieldCurveMissingReason` | 取值二分：`Present(f64)` 或具名缺失 |
| `YieldCurvePoint` | 一条曲线点（`new` 在构造期强制派生完整性） |
| `YieldCurveBatch` | 一批曲线点（`new` 强制批内身份唯一；`points` / `len` / `is_empty`） |

### 校验与守卫

| 项 | 一行语义 |
| --- | --- |
| `validate_curve_point` | 校验单点（派生完整性 + 取值有限性） |
| `validate_curve_batch` | 校验候选切片（逐点 + 批内 `canonical_key` 唯一） |
| `receive_routed_batch` | 接收上游路由来的曲线点并做批校验（不拉取） |
| `kernel_owns` | kernel 能力判定（只拥有 L0 值对象与批校验） |
| `guard_provider_adapter_scope` | provider 授权守卫（恒拒绝：kernel 不得批授权） |
| `YieldCurveRouteSource` / `ROUTE_SOURCES` | 路由接收端来源（4 个） |
| `YieldCurveCapability` | 能力项枚举（含 HTTP / 认证 / 缓存 / 再分发） |
| `PROVIDER_ADAPTERS_PLANNED` / `PROVIDER_ADAPTERS_IMPLEMENTED` | 32 / 0 |

### 解析与 publication

| 项 | 一行语义 |
| --- | --- |
| `parse_yield_curve_batch` | 解析合成夹具 JSON → 曲线批（要求 `_synthetic = true`） |
| `TimePrecision` / `AvailabilityEvidence` / `PitEligibility` | 三元组枚举 |
| `yield_curve_publication_semantics` | 恒返回 `(Date, Inferred, NotEligible)` |
| `is_formal_pit_eligible` | 恒为 `false` |

## 安全与边界语义

- 零 HTTP 客户端、零端点字面量、零凭据读取、零跨仓 `path` 依赖。
- 批内 `canonical_key` 唯一性由构造入口强制；不完整派生点被拒绝。
- 解析器只接受字符串；未知字段 / 缺字段 / 令牌未知一律原子失败；错误消息不回显原始内容。
- kernel 不拥有 HTTP / 认证 / 缓存 / 再分发；不批任何 provider 授权。

## 最小用法

```rust
use yieldx::{Date, Frequency, Unit, YieldCurveBatch, YieldCurveCode, YieldCurveConvention,
             YieldCurveKind, YieldCurvePoint, YieldCurvePointIdentity, YieldCurvePointOrigin,
             YieldCurveRate, YieldCurveTenor};

let point = YieldCurvePoint::new(
    YieldCurvePointIdentity {
        source: YieldCurveCode::new("us_treasury_yield")?,
        series: YieldCurveCode::new("par_yield")?,
        currency: YieldCurveCode::new("USD")?,
        valuation_date: Date::parse("2026-08-14")?,
        maturity: YieldCurveTenor::Y10,
        curve_kind: YieldCurveKind::Nominal,
        vintage: None,
    },
    YieldCurveRate::Present(4.25),
    YieldCurvePointOrigin::Official,
    YieldCurveConvention::new("act365f")?,
    None,
)?;

let batch = YieldCurveBatch::new(vec![point], Frequency::Daily, Unit::Percent)?;
assert_eq!(batch.len(), 1);
# Ok::<(), yieldx::YieldCurveError>(())
```
