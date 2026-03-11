use tracey_api::ApiRule;

/// 要件IDの prefix に応じて、必要な coverage を判定する共通ポリシー。
pub fn rule_needs_impl(rule_id: &str) -> bool {
    !rule_id.starts_with("verify.")
}

pub fn rule_needs_verify(rule_id: &str) -> bool {
    !rule_id.starts_with("impl.")
}

pub fn rule_impl_is_covered(rule: &ApiRule) -> bool {
    rule_needs_impl(&rule.id.base) && !rule.is_stale && !rule.impl_refs.is_empty()
}

pub fn rule_verify_is_covered(rule: &ApiRule) -> bool {
    rule_needs_verify(&rule.id.base) && !rule.verify_refs.is_empty()
}

pub fn rule_impl_is_stale(rule: &ApiRule) -> bool {
    rule_needs_impl(&rule.id.base) && rule.is_stale
}

pub fn rule_missing_impl(rule: &ApiRule) -> bool {
    rule_needs_impl(&rule.id.base) && rule.impl_refs.is_empty()
}

pub fn rule_missing_verify(rule: &ApiRule) -> bool {
    rule_needs_verify(&rule.id.base) && rule.verify_refs.is_empty()
}

pub fn rule_is_fully_covered(rule: &ApiRule) -> bool {
    let impl_ok = !rule_needs_impl(&rule.id.base) || rule_impl_is_covered(rule);
    let verify_ok = !rule_needs_verify(&rule.id.base) || rule_verify_is_covered(rule);
    impl_ok && verify_ok
}

pub fn rule_display_status(rule: &ApiRule) -> &'static str {
    if rule_impl_is_stale(rule) {
        return "stale";
    }

    let has_impl_signal = rule_needs_impl(&rule.id.base) && !rule.impl_refs.is_empty();
    let has_verify_signal = rule_needs_verify(&rule.id.base) && !rule.verify_refs.is_empty();

    if rule_is_fully_covered(rule) {
        "covered"
    } else if has_impl_signal || has_verify_signal {
        "partial"
    } else {
        "uncovered"
    }
}

pub fn coverage_percent(covered: usize, total: usize) -> f64 {
    if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    }
}
