//! yieldx 的基础值对象：日期 / 期间 / 频率 / 单位，以及 L0 曲线值对象的分发入口。
//!
//! 本模块只表达**身份与量纲**：不引入日期库、不做单位换算、不做任何派生计算
//! （利差 / 期限结构拟合 / 插值 / z-score 全部归 analytics）。

use crate::error::{YieldCurveError, YieldCurveResult};

mod curve;
mod identity;
mod tenor;

pub use curve::{
    validate_curve_batch, validate_curve_point, YieldCurveBatch, YieldCurveMissingReason,
    YieldCurvePoint, YieldCurvePointOrigin, YieldCurveRate,
};
pub use identity::{YieldCurveCode, YieldCurvePointIdentity};
pub use tenor::{YieldCurveConvention, YieldCurveKind, YieldCurveTenor, TENOR_CATALOG};

// ---------------------------------------------------------------------------
// 日期与期间
// ---------------------------------------------------------------------------

/// 日历日期（严格 ISO `YYYY-MM-DD` 形态的身份，不含时区与时刻）。
///
/// 字段公开以便调用方直接读取年份 / 月份 / 日；构造 MUST 经 [`Date::new`] 或
/// [`Date::parse`] 校验，否则可能得到非法日期（如 2 月 30 日）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// 年份（4 位形态，取值 1000–9999）。
    pub year: i16,
    /// 月份（1–12）。
    pub month: u8,
    /// 日（按当月天数与闰年规则校验）。
    pub day: u8,
}

impl Date {
    /// 构造并校验一个日期。
    ///
    /// # Errors
    ///
    /// 年份非 4 位形态、月份越界、或日超出该月实际天数时返回
    /// [`YieldCurveError::Invalid`]。
    pub fn new(year: i16, month: u8, day: u8) -> YieldCurveResult<Self> {
        if !(1000..=9999).contains(&year) {
            return Err(YieldCurveError::Invalid(
                "年份须为 4 位形态（1000–9999）".into(),
            ));
        }
        if !(1..=12).contains(&month) {
            return Err(YieldCurveError::Invalid("月份须在 1–12 之间".into()));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(YieldCurveError::Invalid(
                "日超出该月实际天数（已含闰年规则）".into(),
            ));
        }
        Ok(Self { year, month, day })
    }

    /// 解析严格 ISO `YYYY-MM-DD`。
    ///
    /// 月与日 MUST 两位补零；MUST NOT 接受 `2026-2-3`、`2026/02/03` 或带时间部分者。
    ///
    /// # Errors
    ///
    /// 形态不符或日历非法时返回 [`YieldCurveError::Invalid`]。
    ///
    /// # Examples
    ///
    /// ```
    /// use yieldx::Date;
    ///
    /// assert_eq!(Date::parse("2026-08-15")?.month, 8);
    /// assert!(Date::parse("2026-2-3").is_err(), "月/日必须两位补零");
    /// assert!(Date::parse("2026-02-30").is_err(), "日历必须校验");
    /// # Ok::<(), yieldx::YieldCurveError>(())
    /// ```
    pub fn parse(input: &str) -> YieldCurveResult<Self> {
        let b = input.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(YieldCurveError::Invalid(
                "日期须为严格 ISO 形态 YYYY-MM-DD".into(),
            ));
        }
        let year = four_digits(&b[0..4])? as i16;
        let month = two_digits(&b[5..7])?;
        let day = two_digits(&b[8..10])?;
        Self::new(year, month, day)
    }

    /// 以严格 ISO `YYYY-MM-DD` 形态渲染（用于构造规范键）。
    pub(crate) fn to_iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// 该月实际天数（含闰年规则）。
fn days_in_month(year: i16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// 闰年判定：能被 4 整除且（不能被 100 整除 或 能被 400 整除）。
fn is_leap_year(year: i16) -> bool {
    let y = i32::from(year);
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// 解析 4 位 ASCII 数字。
fn four_digits(b: &[u8]) -> YieldCurveResult<u16> {
    if b.len() != 4 || !b.iter().all(u8::is_ascii_digit) {
        return Err(YieldCurveError::Invalid("年份须为 4 位数字".into()));
    }
    Ok(u16::from(b[0] - b'0') * 1000
        + u16::from(b[1] - b'0') * 100
        + u16::from(b[2] - b'0') * 10
        + u16::from(b[3] - b'0'))
}

/// 解析 2 位 ASCII 数字。
fn two_digits(b: &[u8]) -> YieldCurveResult<u8> {
    if b.len() != 2 || !b.iter().all(u8::is_ascii_digit) {
        return Err(YieldCurveError::Invalid("月/日须为 2 位数字".into()));
    }
    Ok((b[0] - b'0') * 10 + (b[1] - b'0'))
}

/// 业务期间的身份。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// 日度期间（曲线估值日的默认期间）。
    Day(Date),
    /// 月度期间。
    Month {
        /// 年份。
        year: i16,
        /// 月份（1–12）。
        month: u8,
    },
    /// 季度期间。
    Quarter {
        /// 年份。
        year: i16,
        /// 季度（1–4）。
        quarter: u8,
    },
    /// 年度期间。
    Year(i16),
    /// 事件型期间（时点事件）。
    Event {
        /// 事件日期。
        date: Date,
    },
}

impl Period {
    /// 复验公开期间分量，避免直接构造绕过日期范围。
    pub(crate) fn validate(&self) -> YieldCurveResult<()> {
        match *self {
            Self::Day(date) | Self::Event { date } => {
                Date::new(date.year, date.month, date.day).map(|_| ())
            }
            Self::Month { year, month } => Date::new(year, month, 1).map(|_| ()),
            Self::Quarter { year, quarter } => {
                Date::new(year, 1, 1)?;
                if !(1..=4).contains(&quarter) {
                    return Err(YieldCurveError::Invalid("季度须在 1–4 之间".into()));
                }
                Ok(())
            }
            Self::Year(year) => Date::new(year, 1, 1).map(|_| ()),
        }
    }

    /// 解析期间形态：`YYYY-MM-DD` → [`Period::Day`]、`YYYY-MM` → [`Period::Month`]、
    /// `YYYY-Qn` → [`Period::Quarter`]、`YYYY` → [`Period::Year`]。
    ///
    /// # Errors
    ///
    /// 形态不属上述四种、或字段越界时返回 [`YieldCurveError::Invalid`]。
    pub fn parse(input: &str) -> YieldCurveResult<Self> {
        let b = input.as_bytes();
        let period = match b.len() {
            10 => Ok(Self::Day(Date::parse(input)?)),
            7 if b[4] == b'-' && (b[5] == b'Q' || b[5] == b'q') => {
                let year = four_digits(&b[0..4])? as i16;
                let quarter = one_digit(&b[6..7])?;
                if !(1..=4).contains(&quarter) {
                    return Err(YieldCurveError::Invalid("季度须在 1–4 之间".into()));
                }
                Ok(Self::Quarter { year, quarter })
            }
            7 if b[4] == b'-' => {
                let year = four_digits(&b[0..4])? as i16;
                let month = two_digits(&b[5..7])?;
                if !(1..=12).contains(&month) {
                    return Err(YieldCurveError::Invalid("月份须在 1–12 之间".into()));
                }
                Ok(Self::Month { year, month })
            }
            4 => Ok(Self::Year(four_digits(b)? as i16)),
            _ => Err(YieldCurveError::Invalid(
                "期间须为 YYYY / YYYY-MM / YYYY-Qn / YYYY-MM-DD 之一".into(),
            )),
        }?;
        period.validate()?;
        Ok(period)
    }
}

/// 解析 1 位 ASCII 数字。
fn one_digit(b: &[u8]) -> YieldCurveResult<u8> {
    if b.len() != 1 || !b[0].is_ascii_digit() {
        return Err(YieldCurveError::Invalid("季度须为 1 位数字".into()));
    }
    Ok(b[0] - b'0')
}

/// 观测频率。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    /// 日频（曲线估值日的常态）。
    Daily,
    /// 周频。
    Weekly,
    /// 月频。
    Monthly,
    /// 季频。
    Quarterly,
    /// 年频。
    Annual,
    /// 事件频（不定期发生时点）。
    Event,
    /// 不规则。
    Irregular,
}

/// 源侧单位。**保留源单位**，换算归下游 Normalize，本层不做。
///
/// 曲线点以百分数（`Percent`）为常态；`Undeclared` 表示上游未声明单位，
/// MUST NOT 静默假定为百分数。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    /// 百分数（曲线利率的常态口径）。
    Percent,
    /// 基点。
    BasisPoint,
    /// 上游未声明单位。
    Undeclared,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parses_strict_iso_only() {
        assert_eq!(
            Date::parse("2026-08-15").expect("合法日期"),
            Date {
                year: 2026,
                month: 8,
                day: 15
            }
        );
        for bad in [
            "2026-2-3",
            "2026/08/15",
            "2026-08-15T00:00:00",
            "20260815",
            "2026-13-01",
            "2026-02-29",
            "",
        ] {
            assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
        }
    }

    #[test]
    fn date_honours_leap_year_rules() {
        assert!(Date::parse("2024-02-29").is_ok(), "2024 是闰年");
        assert!(Date::parse("2000-02-29").is_ok(), "2000 是 400 倍数闰年");
        assert!(Date::parse("1900-02-29").is_err(), "1900 非闰年");
    }

    #[test]
    fn period_parses_four_forms_and_rejects_others() {
        assert!(matches!(Period::parse("2026-08-15"), Ok(Period::Day(_))));
        assert!(matches!(
            Period::parse("2026-08"),
            Ok(Period::Month { month: 8, .. })
        ));
        assert!(matches!(
            Period::parse("2026-Q3"),
            Ok(Period::Quarter { quarter: 3, .. })
        ));
        assert_eq!(Period::parse("2026").expect("年度期间"), Period::Year(2026));
        for bad in ["2026-13", "2026-Q5", "26-08", "2026-8", "2026/08"] {
            assert!(Period::parse(bad).is_err(), "{bad} 必须被拒绝");
        }
    }

    #[test]
    fn date_iso_rendering_is_zero_padded() {
        assert_eq!(
            Date::new(2026, 8, 5).expect("合法日期").to_iso_string(),
            "2026-08-05"
        );
    }

    #[test]
    fn adversarial_period_year_range_is_consistent() {
        for period in ["0000", "0999", "0000-01", "0999-Q1"] {
            assert!(Period::parse(period).is_err(), "非法年份：{period}");
        }
    }
}
