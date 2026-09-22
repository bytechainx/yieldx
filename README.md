# yieldx

`yieldx` 是**收益率曲线 kernel**：把曲线域的 L0 值对象与批校验落成独立、可编译、可测的 Rust 类型库。

**这不是数据采集器，也不是曲线构建器。** kernel 只拥有「身份 + 校验」两件事。

- `kind = kernel` · `access_mode = none` · `authorization = not_applicable`（**没有授权面**）
- 只提供 L0 值对象与校验：期限 / 曲线种类 / 利率惯例 / 曲线点身份 / 官方与派生二分 / 曲线批
- **不拥有** HTTP / 认证 / 缓存 / 再分发
- 32 个 provider 是**规划目录（0/32）**，kernel MUST NOT 据此批任何 provider 授权
- 零内部耦合：不依赖任何 `bytechainx/*` crate，无 `path` 依赖

`production_decision = NO-GO`；`implementation_status = skeleton`（清单 COMPLETE ≠ ship）。

## 安装

本 crate **不发布到 crates.io**，通过 git 依赖引入：

```toml
[dependencies]
yieldx = { git = "https://github.com/bytechainx/yieldx" }
```

本 crate 无 `path` 依赖，引入后可直接构建。

## 用法示例

构造一条曲线点并组成曲线批（批内 `canonical_key` 必须唯一）：

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
assert_eq!(batch.points()[0].identity.maturity, YieldCurveTenor::Y10);
# Ok::<(), yieldx::YieldCurveError>(())
```

解析一份合成夹具（未标注 `_synthetic` 的输入会被拒绝）：

```rust
use yieldx::parse_yield_curve_batch;

let document = include_str!("../tests/fixtures/us_sovereign_par_official.json");
let batch = parse_yield_curve_batch(document)?;
assert_eq!(batch.len(), 3);
# Ok::<(), yieldx::YieldCurveError>(())
```

拒绝规则一览：

- 批内重复 `canonical_key` → `SemanticallyRejected`
- 派生点未声明输入（不完整派生）→ `SemanticallyRejected`
- 官方点却声明了派生输入 → `SemanticallyRejected`
- 请求 kernel 批 provider 授权 → `WriteAuthorityDenied`

## 主要内容

| 类型 | 作用 |
| --- | --- |
| `YieldCurveTenor` / `TENOR_CATALOG` | 11 个标准期限，声明顺序即到期递增顺序（带排序语义） |
| `YieldCurveKind` | 曲线种类三态：名义 / 实际 / OIS |
| `YieldCurveConvention` | 利率惯例（取值域由上游声明，本层不虚构全集） |
| `YieldCurvePointIdentity` | 身份七元组 + 稳定 `canonical_key` |
| `YieldCurvePointOrigin` | 官方点 / 派生点二分 |
| `YieldCurveRate` / `YieldCurveMissingReason` | 取值或具名缺失 |
| `YieldCurvePoint` / `YieldCurveBatch` | 曲线点与曲线批（构造期即校验） |
| `validate_curve_point` / `validate_curve_batch` | 校验入口 |
| `receive_routed_batch` / `YieldCurveRouteSource` | 路由接收端声明（treasuryx DS06/DS07、ecbx YC dataflow、ukx S07） |
| `kernel_owns` / `guard_provider_adapter_scope` | 能力边界与 provider 授权守卫 |
| `parse_yield_curve_batch` | 合成夹具 JSON → 曲线批 |

## 非目标

- **不做曲线构建**：bootstrapping / 插值 / 拟合不在本域
- **不做派生指标**：利差 / z-score / 期限溢价归 analytics
- **不做采集**：`access_mode = none`，HTTP / 认证 / 缓存 / 再分发均不拥有
- **不为 32 个 provider 建立适配**（0/32，规划目录 ≠ 授权清单）
- **不批任何 provider 授权**：`guard_provider_adapter_scope` 恒拒绝
- **不拥有存储与分发**

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

三类测试（`tests/tdd_contracts.rs` / `tests/sdd_spec.rs` / `tests/aidd_boundary.rs`）全部离线运行，
不访问外网、不读环境变量。`tests/fixtures/` 下的夹具全部为**合成样本**，不是真实源数据，
不构成任何证据。

## 许可

MIT OR Apache-2.0
