/// Markdown 本文から参照される GitHub 互換の要件アンカーを作る。
pub(crate) fn github_requirement_anchor(prefix: &str, rule_id: &str) -> String {
    github_markdown_anchor(&format!("{prefix}[{rule_id}]"))
}

fn github_markdown_anchor(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::github_requirement_anchor;

    #[test]
    fn preserves_underscores_like_github_heading_anchor() {
        assert_eq!(
            github_requirement_anchor("r", "req.server_list.load.yaml"),
            "rreqserver_listloadyaml"
        );
    }

    #[test]
    fn removes_requirement_punctuation() {
        assert_eq!(
            github_requirement_anchor("r", "req.gateway.start"),
            "rreqgatewaystart"
        );
    }
}
