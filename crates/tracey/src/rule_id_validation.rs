use tracey_core::RuleId;

/// 要件 ID の命名規約を判定する。
pub(crate) fn is_valid_rule_id(id: &RuleId) -> bool {
    for segment in id.base.split('.') {
        if segment.is_empty() {
            return false;
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return false;
        }
        if !segment
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::is_valid_rule_id;
    use tracey_core::parse_rule_id;

    #[test]
    fn allows_underscore_inside_segment() {
        let id = parse_rule_id("req.server_list.load.yaml").expect("valid rule id");
        assert!(is_valid_rule_id(&id));
    }

    #[test]
    fn still_rejects_leading_underscore_segment() {
        let id = parse_rule_id("req._server.load").expect("rule id should parse");
        assert!(!is_valid_rule_id(&id));
    }

    #[test]
    fn still_rejects_uppercase_segment() {
        let id = parse_rule_id("req.Server.load").expect("rule id should parse");
        assert!(!is_valid_rule_id(&id));
    }
}
