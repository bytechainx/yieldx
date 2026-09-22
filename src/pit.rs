//! yieldx 的 publication 语义：时间精度、可得性证据层与正式 PIT 资格。
//!
//! kernel 与 10 个数据源一致：publication 时刻**全部**属推断层，故判定恒为
//! [`TimePrecision::Date`] + [`AvailabilityEvidence::Inferred`] +
//! [`PitEligibility::NotEligible`]；不得补造 `00:00 UTC` 之类时刻把 `Date`
//! 伪装成 `Instant`，也不得静默升格。

/// 时间精度：源只给日期还是给出时刻。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimePrecision {
    /// 只有日期。
    Date,
    /// 有明确时刻。
    Instant,
}

/// 可得性证据层：官方字段 > 日历 > 推断。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvailabilityEvidence {
    /// 官方字段。
    Official,
    /// 发布日历。
    Calendar,
    /// 推断。
    Inferred,
}

/// 正式 PIT 资格。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitEligibility {
    /// 可进正式 PIT。
    Formal,
    /// 不可进正式 PIT。
    NotEligible,
}

/// yieldx 的 publication 语义三元组。
///
/// 返回值**恒为** `(Date, Inferred, NotEligible)`。
#[must_use]
pub fn yield_curve_publication_semantics() -> (TimePrecision, AvailabilityEvidence, PitEligibility)
{
    (
        TimePrecision::Date,
        AvailabilityEvidence::Inferred,
        PitEligibility::NotEligible,
    )
}

/// 是否具备正式 PIT 资格。本层恒为 `false`（清单 `is_formal_pit_eligible()` 口径）。
#[must_use]
pub fn is_formal_pit_eligible() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_triple_is_fixed() {
        assert_eq!(
            yield_curve_publication_semantics(),
            (
                TimePrecision::Date,
                AvailabilityEvidence::Inferred,
                PitEligibility::NotEligible
            )
        );
    }

    #[test]
    fn formal_pit_eligibility_is_pinned_to_not_eligible() {
        let (precision, evidence, eligibility) = yield_curve_publication_semantics();
        assert_eq!(precision, TimePrecision::Date);
        assert_eq!(evidence, AvailabilityEvidence::Inferred);
        assert_eq!(eligibility, PitEligibility::NotEligible);
        assert!(!is_formal_pit_eligible(), "不得静默升格为正式 PIT");
    }
}
