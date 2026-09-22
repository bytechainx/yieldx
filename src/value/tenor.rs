//! 期限 / 曲线种类 / 利率惯例：曲线点的三个**结构性**身份维度。
//!
//! 三者都是清单 §1.1 明列的 kernel 值对象。期限带**排序语义**（按到期月数递增）；
//! 曲线种类只有清单提到的三态（名义 / 实际 / OIS）；惯例的取值域清单**未钉死**，
//! 故本层只做身份合法性校验，MUST NOT 虚构惯例全集。

use crate::error::{YieldCurveError, YieldCurveResult};

/// 标准期限。
///
/// 变体的**声明顺序即到期月数递增顺序**（派生 `Ord` 因此具有排序语义），
/// 该不变量由 `tenor_order_matches_maturity_months` 用例钉死。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum YieldCurveTenor {
    /// 1 个月。
    M1,
    /// 3 个月。
    M3,
    /// 6 个月。
    M6,
    /// 1 年。
    Y1,
    /// 2 年。
    Y2,
    /// 3 年。
    Y3,
    /// 5 年。
    Y5,
    /// 7 年。
    Y7,
    /// 10 年。
    Y10,
    /// 20 年。
    Y20,
    /// 30 年。
    Y30,
}

impl YieldCurveTenor {
    /// 到期月数。
    #[must_use]
    pub fn months(self) -> u32 {
        match self {
            Self::M1 => 1,
            Self::M3 => 3,
            Self::M6 => 6,
            Self::Y1 => 12,
            Self::Y2 => 24,
            Self::Y3 => 36,
            Self::Y5 => 60,
            Self::Y7 => 84,
            Self::Y10 => 120,
            Self::Y20 => 240,
            Self::Y30 => 360,
        }
    }

    /// 期限的规范标签（**大小写敏感**，避免静默等值）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::M1 => "1M",
            Self::M3 => "3M",
            Self::M6 => "6M",
            Self::Y1 => "1Y",
            Self::Y2 => "2Y",
            Self::Y3 => "3Y",
            Self::Y5 => "5Y",
            Self::Y7 => "7Y",
            Self::Y10 => "10Y",
            Self::Y20 => "20Y",
            Self::Y30 => "30Y",
        }
    }

    /// 解析期限标签（仅接受 [`YieldCurveTenor::label`] 的规范形态）。
    ///
    /// # Errors
    ///
    /// 标签不在规范形态内（含大小写不符）时返回
    /// [`YieldCurveError::SemanticallyRejected`]。
    pub fn parse(input: &str) -> YieldCurveResult<Self> {
        match input {
            "1M" => Ok(Self::M1),
            "3M" => Ok(Self::M3),
            "6M" => Ok(Self::M6),
            "1Y" => Ok(Self::Y1),
            "2Y" => Ok(Self::Y2),
            "3Y" => Ok(Self::Y3),
            "5Y" => Ok(Self::Y5),
            "7Y" => Ok(Self::Y7),
            "10Y" => Ok(Self::Y10),
            "20Y" => Ok(Self::Y20),
            "30Y" => Ok(Self::Y30),
            other => Err(YieldCurveError::SemanticallyRejected(format!(
                "未知的期限标签（长度 {}）；仅接受规范形态且大小写敏感",
                other.len()
            ))),
        }
    }
}

/// 期限全集（顺序与 [`YieldCurveTenor`] 的声明顺序一致）。
pub const TENOR_CATALOG: [YieldCurveTenor; 11] = [
    YieldCurveTenor::M1,
    YieldCurveTenor::M3,
    YieldCurveTenor::M6,
    YieldCurveTenor::Y1,
    YieldCurveTenor::Y2,
    YieldCurveTenor::Y3,
    YieldCurveTenor::Y5,
    YieldCurveTenor::Y7,
    YieldCurveTenor::Y10,
    YieldCurveTenor::Y20,
    YieldCurveTenor::Y30,
];

/// 曲线种类。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldCurveKind {
    /// 名义曲线。
    Nominal,
    /// 实际（通胀调整）曲线。
    Real,
    /// 隔夜指数互换（OIS）曲线。
    Ois,
}

impl YieldCurveKind {
    /// 曲线种类的规范标签。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Real => "real",
            Self::Ois => "ois",
        }
    }

    /// 解析曲线种类标签。
    ///
    /// # Errors
    ///
    /// 标签不在三态内时返回 [`YieldCurveError::SemanticallyRejected`]。
    pub fn parse(input: &str) -> YieldCurveResult<Self> {
        match input {
            "nominal" => Ok(Self::Nominal),
            "real" => Ok(Self::Real),
            "ois" => Ok(Self::Ois),
            other => Err(YieldCurveError::SemanticallyRejected(format!(
                "未知的曲线种类标签（长度 {}）",
                other.len()
            ))),
        }
    }
}

/// 利率惯例。
///
/// 清单未钉死惯例的取值域，故本层只保证「是一个可区分的非空身份」，
/// **不**提供惯例全集，也**不**据此做任何计算。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YieldCurveConvention(String);

impl YieldCurveConvention {
    /// 构造并校验一个惯例标识。
    ///
    /// # Errors
    ///
    /// 空串、前后空白、含控制字符或超过 64 个字符时返回
    /// [`YieldCurveError::Missing`] 或 [`YieldCurveError::Invalid`]。
    pub fn new(value: &str) -> YieldCurveResult<Self> {
        if value.trim().is_empty() {
            return Err(YieldCurveError::Missing("利率惯例不得为空".into()));
        }
        if value.len() > 64 {
            return Err(YieldCurveError::Invalid(
                "利率惯例标识过长（上限 64）".into(),
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(YieldCurveError::Invalid("利率惯例不得含控制字符".into()));
        }
        if value.trim() != value {
            return Err(YieldCurveError::Invalid(
                "利率惯例不得含前后空白（避免静默等值）".into(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// 以字符串切片读取。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenor_order_matches_maturity_months() {
        let mut previous = 0;
        for tenor in TENOR_CATALOG {
            let months = tenor.months();
            assert!(
                months > previous,
                "{} 的到期月数必须严格递增（前值 {previous}）",
                tenor.label()
            );
            previous = months;
        }
        assert!(
            YieldCurveTenor::M1 < YieldCurveTenor::Y30,
            "排序语义按到期递增"
        );
        assert_eq!(TENOR_CATALOG.len(), 11);
    }

    #[test]
    fn tenor_parse_is_case_sensitive_and_round_trips() {
        for tenor in TENOR_CATALOG {
            assert_eq!(
                YieldCurveTenor::parse(tenor.label()).expect("规范标签必须可解析"),
                tenor
            );
        }
        assert!(YieldCurveTenor::parse("10y").is_err(), "大小写必须敏感");
        assert!(YieldCurveTenor::parse("4Y").is_err(), "非规范期限必须拒绝");
        assert!(YieldCurveTenor::parse("").is_err());
    }

    #[test]
    fn curve_kind_covers_three_states() {
        for (label, kind) in [
            ("nominal", YieldCurveKind::Nominal),
            ("real", YieldCurveKind::Real),
            ("ois", YieldCurveKind::Ois),
        ] {
            assert_eq!(
                YieldCurveKind::parse(label).expect("规范标签必须可解析"),
                kind
            );
            assert_eq!(kind.label(), label);
        }
        assert!(
            YieldCurveKind::parse("forward").is_err(),
            "三态之外必须拒绝"
        );
    }

    #[test]
    fn convention_rejects_blank_and_whitespace() {
        assert_eq!(
            YieldCurveConvention::new("act365f")
                .expect("合法惯例")
                .as_str(),
            "act365f"
        );
        assert!(YieldCurveConvention::new("").is_err());
        assert!(YieldCurveConvention::new("  ").is_err());
        assert!(YieldCurveConvention::new(" act365f").is_err());
        assert!(YieldCurveConvention::new("a\tb").is_err());
        assert!(YieldCurveConvention::new(&"x".repeat(65)).is_err());
    }
}
