# Changelog — yieldx

本文件记录 `yieldx` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

本 crate 为特性 005（`005-macro-data-source-crates`）新建，不含从其它工程延续的版本线，
故从 `0.1.0` 起算。

## [Unreleased]

## [0.1.1] - 2026-09-23

### 修复

- 修复宏观数据源对抗审查发现的数值与身份边界缺陷
- 规范键转义身份令牌中的分隔符与百分号，区分缺失 vintage 和实值 `-`，避免不同身份碰撞。
- 离线解析契约加严：合成夹具必须携带非空白 `_note` 说明；缺失、null 或空白均拒绝。此为本仓采用公共契约 §6.2 允许的加严规则。

## [0.1.0] - 2026-09-22

### 新增

- 从 `specs/adapter/yield_curve.md` 的 kernel 义务落地为独立 crate：L0 值对象
  `YieldCurveTenor` / `YieldCurveKind` / `YieldCurveConvention` / `YieldCurvePointIdentity` /
  `YieldCurvePointOrigin` / `YieldCurveBatch`。
- `YieldCurveTenor`：11 个标准期限，声明顺序即到期递增顺序（派生 `Ord` 具排序语义）；
  `parse` 大小写敏感、`months` 给出到期月数。
- `YieldCurveKind`：清单三态（名义 / 实际 / OIS）。
- `YieldCurveConvention`：只做身份合法性校验，不虚构惯例全集。
- `YieldCurvePointIdentity`：身份七元组与稳定 `canonical_key`
  （`vintage` 缺失渲染占位符，维度不得省略）。
- `YieldCurvePointOrigin`：官方点与派生点二分；派生点必须完整（不完整派生拒绝），
  官方点不得声明派生输入。
- `YieldCurveBatch`：批内 `canonical_key` 唯一性由构造入口强制；空批合法、含重复身份非法。
- 校验入口 `validate_curve_point` / `validate_curve_batch`（后者接受候选切片）。
- 路由接收端声明 `receive_routed_batch`：treasuryx DS06/DS07、ecbx YC dataflow、ukx S07；
  只做校验，不拉取、不认证、不缓存、不再分发。
- kernel 能力边界 `kernel_owns` 与 provider 授权守卫 `guard_provider_adapter_scope`
  （恒 `WriteAuthorityDenied`；32 provider 规划目录，0/32）。
- 离线解析 `parse_yield_curve_batch`：JSON 合成夹具 → 曲线批；要求
  `"_synthetic": true`，未知字段与非法令牌原子失败。
- publication 语义：`yield_curve_publication_semantics` 恒为 `(Date, Inferred, NotEligible)`。
- 三类测试：`tests/tdd_contracts.rs`（22 个公开入口的 TDD-PROBE 变异探测表）、
  `tests/sdd_spec.rs`（`docs/标准.md` 全部 7 个 `##` 章节的可执行断言）、
  `tests/aidd_boundary.rs`（8 条经复核的 AI 生成对抗 / 边界用例）。
- 合成夹具 `tests/fixtures/{us_sovereign_par_official,us_with_derived_spread,negative_incomplete_derived}.json`
  （带 `_synthetic` 标注，非真实源数据）。
- 文档面：`README.md` / `docs/API.md` / `docs/标准.md` / `CONTEXT.md` / `CONTRIBUTING.md` / `AGENTS.md`。

### 说明

- **本 crate 没有授权面**：`authorization = not_applicable`，故不存在 `authz` 模块，
  也不导出任何授权类型或判定函数。
