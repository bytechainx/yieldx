# yieldx Agent 指南

> 本文件为 AI Agent 在本仓库工作时的入口指南。

## 项目定位

收益率曲线 **kernel**：只拥有 L0 值对象与批校验。不采集、不构建曲线、不拥有 HTTP / 认证 /
缓存 / 再分发，也不批任何 provider 授权。`authorization = not_applicable`；`production_decision = NO-GO`。

## 技术栈

- Rust edition 2021，rust-version 1.71（由依赖图推导，见 `CONTEXT.md`）
- 依赖：`serde`（derive）+ `serde_json`（解析合成夹具 JSON）、`thiserror` 2（错误模型）
- 无 path 依赖；零业务耦合，不依赖 `kernel` / `contracts` 或任何 `bytechainx/*` crate
- crate 级 lint：`unwrap_used` / `expect_used` / `panic` / `unreachable` / `todo` / `unimplemented`
  全部 `deny`（测试代码经 `cfg_attr(test)` 豁免）

## 代码结构

```text
src/
├── lib.rs               # crate 文档 + 模块声明 + 门面 pub use（**无 authz**）
├── error.rs             # YieldCurveError / YieldCurveErrorKind / YieldCurveResult
├── value.rs             # Date / Period / Frequency / Unit + 子模块分发
├── value/tenor.rs       # YieldCurveTenor / YieldCurveKind / YieldCurveConvention
├── value/identity.rs    # YieldCurveCode / YieldCurvePointIdentity（canonical_key）
├── value/curve.rs       # YieldCurvePoint / YieldCurveBatch / 两个 validate 入口
├── routing.rs           # 路由接收端 + kernel 能力边界 + provider 授权守卫
├── pit.rs               # publication 语义三元组
└── parse.rs             # 只解析显式合成标注 JSON 的离线解析器
```

模块依赖方向单向：`error ← value ← {routing, parse}`、`error ← pit`，无环。禁止 `mod.rs`，
禁止 `utils` / `helpers` / `common` / `manager` / `base` / `global` / `misc` 命名。

## 硬约束（违反即返工）

1. **禁止**创建 `src/authz.rs` 或导出任何授权类型 / 判定函数
   （kernel `authorization = not_applicable`）。
2. **禁止**任何 HTTP 客户端、异步运行时、`chrono` / `time`、`rand`；禁止读环境变量或凭据。
3. **禁止**在源码中出现任何 URL / 端点字面量。
4. **禁止**写入任何 `path = "../<其它仓>"` 依赖或依赖 `bytechainx/*`。
5. **禁止**绕过 `YieldCurveBatch::new` 的批校验引入新的构造路径（批内唯一是硬不变量）。
6. **禁止**实现曲线构建或派生计算（插值 / 拟合 / 利差 / z-score）。
7. **禁止**为 32 个 provider 建任何适配，或借 kernel 批任何 provider 授权。
8. **禁止**把 `tests/fixtures/` 的合成样本表述为「实测」「核验 PASS」或任何证据等级。
9. **禁止**在库代码里 `unwrap` / `expect` / `panic!` / `println!`。

## 门禁四件套

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

## 相关文档

- 采集/边界权威：`specs/adapter/yield_curve.md`（工作区根）
- API 文档：`docs/API.md`
- 能力标准与验收：`docs/标准.md`
- 术语与领域语言：`CONTEXT.md`
- 贡献指南：`CONTRIBUTING.md`
- 变更记录：`CHANGELOG.md`
- 基准测试：`benches/hot_path.rs`
