# yieldx 上下文

本文件定义 `yieldx` 与使用方共享的核心词汇、边界与已知缺口。它只记录领域含义与能力边界，
不记录具体实现、存储或部署决定。

## 角色与边界

**kernel**：曲线域的**身份与校验**层——上游 provider 声明「这是哪条曲线上的哪个点」，
本层保证该声明**身份唯一、语义自洽**。
_Avoid_: provider（kernel 不采集、不认证、不缓存、不再分发，也不批任何 provider 授权）

**L0 值对象**：期限 / 曲线种类 / 惯例 / 身份七元组 / 官方与派生二分 / 曲线批。
_Avoid_: 曲线构建（bootstrapping / 插值 / 拟合属 analytics 或 provider 侧）

**接收端**：曲线点由 `treasuryx` DS06/DS07、`ecbx` YC dataflow、`ukx` S07 路由到本域；
本域只接收并校验，**不拉取**。
_Avoid_: 采集器（本域 `access_mode = none`）

**无授权面**：`authorization = not_applicable`。kernel 不是 provider，因此本库**没有** `authz`
模块、没有授权类型、没有授权判定函数——这不是缺口，而是语义。
_Avoid_: 授权判定（`AuthorizationDenied` 错误分类的存在只为表达「本层不得主张授权」）

## 身份与唯一性

**身份七元组**：`source + series + currency + valuation_date + maturity + curve_kind + vintage`。
任何一个维度变化都必须产生不同的身份。
_Avoid_: 近似匹配（曲线点的「差不多」没有意义，端点插值属下游）

**规范键**（`canonical_key`）：身份的稳定文本投影，字段顺序固定；`vintage` 缺失渲染为 `-`
（**保留维度**）。批内唯一性以它为准。
_Avoid_: 复合 `Hash`（规范键是可读、可日志、可跨语言比对的形态）

**批内唯一**：重复身份 MUST 被拒绝，不得静默去重、不得后写覆盖。
_Avoid_: 最后写入胜出（覆盖会让「哪一条是源事实」不可判定）

## 官方与派生

**官方点**：上游直接给出的点；MUST NOT 声明派生输入。
**派生点**：由其它曲线点计算得到的点；MUST 声明至少一个输入身份。
_Avoid_: 把派生点当官方点（不完整派生正是清单负向样本表达的缺陷）

**具名缺失**：`YieldCurveRate::Missing(reason)` 区分「上游空」「尚未发布」「不适用」。
_Avoid_: 补 0 / 补前值 / 线性插值补洞（静默填充会伪造源事实）

## rust-version 推导

规则：`rust-version` = 依赖图中所有依赖所声明 `rust_version` 的最大值
（`cargo metadata --format-version 1` 逐包读取）。

| 依赖 | 版本 | 其 `rust_version` |
| --- | --- | --- |
| `serde` / `serde_core` | 1.0.229 | 1.56 |
| `serde_derive` / `proc-macro2` / `quote` / `syn` | 1.0.229 / 1.0.107 / 1.0.47 / 3.0.6 | 1.71 |
| `serde_json` | 1.0.151 | 1.71 |
| `thiserror` / `thiserror-impl` | 2.0.20 | 1.71 |
| `itoa` | 1.0.18 | 1.68 |
| `memchr` | 2.8.3 | 1.61 |
| `ryu` / `unicode-ident` / `zmij` | 1.0.23 / 1.0.26 / 1.0.23 | 1.71 |

**最大值 = 1.71**，故本 crate 声明 `rust-version = "1.71"`（不得照抄兄弟仓）。
本 crate 无 dev-dependency 之外的额外依赖，无 `path` 依赖。

## 已知缺口

1. **未实现采集**：`access_mode = none`；HTTP / 认证 / 缓存 / 再分发均不拥有，
   由 `kernel_owns` 与 `guard_provider_adapter_scope` 在类型层拒绝。
2. **32 provider 适配为 0/32**：规划目录不等于授权清单；本轮只声明接收端语义。
3. **未实现曲线构建**：bootstrapping / 插值 / 拟合、利差、z-score 全部归 analytics。
4. **无官方 vintage 面**：`vintage` 恒可为 `None`；若日后接入官方 vintage，
   须同步更新清单与契约后再改本库，**禁止静默升格**。
5. **惯例取值域未钉死**：`YieldCurveConvention` 只保证身份合法，不提供惯例全集。
6. **夹具为合成样本**：`tests/fixtures/` 全部为自拟样本，不是真实源数据，不构成任何证据；
   解析入口要求 `_synthetic = true` 以免把来源不明的数据当作源事实。
