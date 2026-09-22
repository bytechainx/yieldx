//! 曲线点身份：上游声明的**身份七元组**。
//!
//! 身份 = `source + series + currency + valuation_date + maturity + curve_kind + vintage`。
//! 规范键 [`YieldCurvePointIdentity::canonical_key`] 是该身份的稳定文本投影，
//! 批内唯一性校验以它为判据（见 `value::curve`）。
//!
//! `source` / `series` / `currency` / `vintage` 都是**上游声明的身份令牌**，
//! 本层只校验「是一个可区分的非空身份」，MUST NOT 虚构取值域，也不做币种/来源白名单。

use crate::error::{YieldCurveError, YieldCurveResult};
use crate::value::{Date, YieldCurveKind, YieldCurveTenor};

/// 上游声明的身份令牌。
///
/// 同一类型用于 `source` / `series` / `currency` / `vintage` 四个字段；
/// 四者的**语义由字段名承载**，规范键按固定标签顺序渲染，故互换字段会得到不同的键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YieldCurveCode(String);

impl YieldCurveCode {
    /// 构造并校验一个身份令牌。
    ///
    /// # Errors
    ///
    /// 空串、前后空白、含控制字符或超过 64 个字符时返回
    /// [`YieldCurveError::Missing`] 或 [`YieldCurveError::Invalid`]。
    pub fn new(value: &str) -> YieldCurveResult<Self> {
        if value.trim().is_empty() {
            return Err(YieldCurveError::Missing("身份令牌不得为空".into()));
        }
        if value.len() > 64 {
            return Err(YieldCurveError::Invalid("身份令牌过长（上限 64）".into()));
        }
        if value.chars().any(char::is_control) {
            return Err(YieldCurveError::Invalid("身份令牌不得含控制字符".into()));
        }
        if value.trim() != value {
            return Err(YieldCurveError::Invalid(
                "身份令牌不得含前后空白（避免静默等值）".into(),
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

/// 曲线点身份（批内唯一性以 [`YieldCurvePointIdentity::canonical_key`] 为准）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YieldCurvePointIdentity {
    /// 上游来源标识。
    pub source: YieldCurveCode,
    /// 上游序列标识。
    pub series: YieldCurveCode,
    /// 币种标识。
    pub currency: YieldCurveCode,
    /// 估值日。
    pub valuation_date: Date,
    /// 到期期限。
    pub maturity: YieldCurveTenor,
    /// 曲线种类。
    pub curve_kind: YieldCurveKind,
    /// 版本标识；无官方 vintage 面时 MUST 为 `None`，MUST NOT 伪造。
    pub vintage: Option<YieldCurveCode>,
}

impl YieldCurvePointIdentity {
    /// 规范键：身份的稳定文本投影（字段顺序固定，`vintage` 缺失渲染为 `-`）。
    ///
    /// 同一身份 MUST 得到同一键；不同身份 MUST 得到不同键。
    /// 令牌中的百分号、分号和等号按百分号编码；实值 vintage `-` 编为 `%2D`。
    #[must_use]
    pub fn canonical_key(&self) -> String {
        format!(
            "source={};series={};currency={};valuation_date={};maturity={};curve_kind={};vintage={}",
            escape_code(self.source.as_str()),
            escape_code(self.series.as_str()),
            escape_code(self.currency.as_str()),
            self.valuation_date.to_iso_string(),
            self.maturity.label(),
            self.curve_kind.label(),
            self.vintage.as_ref().map_or_else(|| "-".into(), |v| {
                if v.as_str() == "-" { "%2D".into() } else { escape_code(v.as_str()) }
            }),
        )
    }
}

/// 转义标签分隔符与转义符，保留普通令牌的既有文本形态。
fn escape_code(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace('=', "%3D")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(value: &str) -> YieldCurveCode {
        YieldCurveCode::new(value).expect("合法令牌")
    }

    fn identity() -> YieldCurvePointIdentity {
        YieldCurvePointIdentity {
            source: code("treasury"),
            series: code("DS06"),
            currency: code("USD"),
            valuation_date: Date::new(2026, 8, 14).expect("合法日期"),
            maturity: YieldCurveTenor::Y10,
            curve_kind: YieldCurveKind::Nominal,
            vintage: None,
        }
    }

    #[test]
    fn canonical_key_is_stable_and_fully_qualified() {
        let key = identity().canonical_key();
        assert_eq!(
            key,
            "source=treasury;series=DS06;currency=USD;valuation_date=2026-08-14;maturity=10Y;curve_kind=nominal;vintage=-"
        );
        assert_eq!(identity().canonical_key(), key, "同身份必须同键");
    }

    #[test]
    fn every_identity_dimension_changes_the_key() {
        let base = identity().canonical_key();

        let mut other = identity();
        other.source = code("ecb");
        assert_ne!(other.canonical_key(), base, "source 必须参与身份");

        let mut other = identity();
        other.series = code("DS07");
        assert_ne!(other.canonical_key(), base, "series 必须参与身份");

        let mut other = identity();
        other.currency = code("EUR");
        assert_ne!(other.canonical_key(), base, "currency 必须参与身份");

        let mut other = identity();
        other.valuation_date = Date::new(2026, 8, 15).expect("合法日期");
        assert_ne!(other.canonical_key(), base, "valuation_date 必须参与身份");

        let mut other = identity();
        other.maturity = YieldCurveTenor::Y30;
        assert_ne!(other.canonical_key(), base, "maturity 必须参与身份");

        let mut other = identity();
        other.curve_kind = YieldCurveKind::Real;
        assert_ne!(other.canonical_key(), base, "curve_kind 必须参与身份");

        let mut other = identity();
        other.vintage = Some(code("v2"));
        assert_ne!(other.canonical_key(), base, "vintage 必须参与身份");
    }

    #[test]
    fn delimiter_and_escape_tokens_cannot_collide() {
        let mut left = identity();
        left.source = code("a;series=b");
        left.series = code("c");
        let mut right = identity();
        right.source = code("a");
        right.series = code("b;series=c");
        assert_ne!(left.canonical_key(), right.canonical_key());

        right = left.clone();
        right.source = code("a%3Bseries%3Db");
        assert_ne!(left.canonical_key(), right.canonical_key());
    }

    #[test]
    fn literal_vintage_placeholder_is_not_missing_vintage() {
        let absent = identity();
        let mut literal = absent.clone();
        literal.vintage = Some(code("-"));
        assert_ne!(absent.canonical_key(), literal.canonical_key());
        literal.vintage = Some(code("%2D"));
        let mut dash = absent.clone();
        dash.vintage = Some(code("-"));
        assert_ne!(literal.canonical_key(), dash.canonical_key());
    }

    #[test]
    fn code_rejects_blank_and_overlong() {
        assert!(YieldCurveCode::new("").is_err());
        assert!(YieldCurveCode::new("  ").is_err());
        assert!(YieldCurveCode::new(" usd").is_err());
        assert!(YieldCurveCode::new("a\nb").is_err());
        assert!(YieldCurveCode::new(&"x".repeat(65)).is_err());
    }
}
