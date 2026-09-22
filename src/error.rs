//! yieldx 的错误分类与错误类型。
//!
//! 分类按「调用方应如何反应」划分，使调用方无需字符串匹配即可分流
//! （授权拒绝 / 路由拒绝 / 越权写入 / 语义拒绝四类必须可区分）。
//!
//! kernel **没有**授权面（`authorization = not_applicable`），但错误分类仍保留
//! `AuthorizationDenied` 与 `WriteAuthorityDenied`：它们用于表达
//! 「本层不得主张 provider 授权」「写入主权不在本域」这两类拒绝。

/// 错误分类：按「调用方应如何反应」划分。
///
/// 禁止用字符串匹配替代对本枚举的匹配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurveErrorKind {
    /// 输入形态或取值非法（调用方应修数据，重试无意义）。
    Invalid,
    /// 缺少必需项。
    Missing,
    /// 授权判定未通过（fail-closed 落点）。
    AuthorizationDenied,
    /// 该产品不属于本域，已被路由规则拒绝。
    RoutedElsewhere,
    /// 越权写入（权威写入归他域）。
    WriteAuthorityDenied,
    /// 结构可解析但语义不被接受（如批内重复身份、派生输入不完整）。
    SemanticallyRejected,
    /// 尚未实现的规划能力。
    NotApplicable,
    /// 不变量被破坏（库内 bug 的信号）。
    Invariant,
}

/// yieldx 错误。保留可区分的分类与来源链。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum YieldCurveError {
    /// 输入非法。
    #[error("输入非法：{0}")]
    Invalid(String),
    /// 缺少必需项。
    #[error("缺少必需项：{0}")]
    Missing(String),
    /// 授权判定未通过。
    #[error("授权判定未通过：{0}")]
    AuthorizationDenied(String),
    /// 产品不属于本域。
    #[error("不属于本域，已路由他处：{0}")]
    RoutedElsewhere(String),
    /// 越权写入。
    #[error("越权写入被拒绝：{0}")]
    WriteAuthorityDenied(String),
    /// 语义拒绝。
    #[error("语义不被接受：{0}")]
    SemanticallyRejected(String),
    /// 规划能力未实现。
    #[error("规划能力尚未实现：{0}")]
    NotApplicable(String),
    /// 不变量被破坏。
    #[error("不变量被破坏：{0}")]
    Invariant(String),
}

impl YieldCurveError {
    /// 分类，供调用方按「如何反应」分流。
    #[must_use]
    pub fn kind(&self) -> YieldCurveErrorKind {
        match self {
            Self::Invalid(_) => YieldCurveErrorKind::Invalid,
            Self::Missing(_) => YieldCurveErrorKind::Missing,
            Self::AuthorizationDenied(_) => YieldCurveErrorKind::AuthorizationDenied,
            Self::RoutedElsewhere(_) => YieldCurveErrorKind::RoutedElsewhere,
            Self::WriteAuthorityDenied(_) => YieldCurveErrorKind::WriteAuthorityDenied,
            Self::SemanticallyRejected(_) => YieldCurveErrorKind::SemanticallyRejected,
            Self::NotApplicable(_) => YieldCurveErrorKind::NotApplicable,
            Self::Invariant(_) => YieldCurveErrorKind::Invariant,
        }
    }

    /// 是否值得重试。本层无网络，除 `Invariant` 外一律 `false`。
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), YieldCurveErrorKind::Invariant)
    }
}

/// 本 crate 统一结果别名。
pub type YieldCurveResult<T> = Result<T, YieldCurveError>;

#[cfg(test)]
mod tests {
    use super::*;

    fn all_variants() -> Vec<YieldCurveError> {
        vec![
            YieldCurveError::Invalid("a".into()),
            YieldCurveError::Missing("b".into()),
            YieldCurveError::AuthorizationDenied("c".into()),
            YieldCurveError::RoutedElsewhere("d".into()),
            YieldCurveError::WriteAuthorityDenied("e".into()),
            YieldCurveError::SemanticallyRejected("f".into()),
            YieldCurveError::NotApplicable("g".into()),
            YieldCurveError::Invariant("h".into()),
        ]
    }

    #[test]
    fn kind_maps_every_variant_distinctly() {
        let kinds: Vec<YieldCurveErrorKind> =
            all_variants().iter().map(YieldCurveError::kind).collect();
        let unique: std::collections::HashSet<_> = kinds.iter().collect();
        assert_eq!(unique.len(), kinds.len(), "每个变体须映射到互不相同的分类");
    }

    #[test]
    fn only_invariant_is_retryable() {
        for error in all_variants() {
            let expected = error.kind() == YieldCurveErrorKind::Invariant;
            assert_eq!(error.is_retryable(), expected, "分类 {:?}", error.kind());
        }
    }

    #[test]
    fn display_is_non_empty_and_source_chain_present() {
        for error in all_variants() {
            assert!(!error.to_string().is_empty());
        }
        let as_std: &dyn std::error::Error = &YieldCurveError::Invalid("x".into());
        assert!(as_std.source().is_none());
    }

    #[test]
    fn non_kernel_capability_denial_is_readable_and_carries_no_endpoint() {
        let error = crate::routing::guard_provider_adapter_scope().expect_err("kernel 不得批授权");
        assert_eq!(error.kind(), YieldCurveErrorKind::WriteAuthorityDenied);
        let message = error.to_string();
        assert!(!message.is_empty());
        assert!(
            !message.contains("http") && !message.contains("://"),
            "错误消息不得含端点字面量：{message}"
        );
    }
}
