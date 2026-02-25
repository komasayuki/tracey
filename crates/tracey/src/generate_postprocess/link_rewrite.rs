/// HTML内の `/...` 形式リンクを file:// でも辿れる相対リンクへ変換する。
pub(super) fn rewrite_html_root_links(input: &str, prefix: &str) -> String {
    let href = rewrite_html_attr_links(input, "href", prefix, true);
    let src = rewrite_html_attr_links(&href, "src", prefix, false);
    rewrite_html_attr_links(&src, "action", prefix, false)
}

fn rewrite_html_attr_links(input: &str, attr: &str, prefix: &str, force_index: bool) -> String {
    let mut out = String::with_capacity(input.len() + 64);
    let dq_pat = format!(r#"{attr}=""#);
    let sq_pat = format!("{attr}='");
    let mut cursor = 0usize;

    while cursor < input.len() {
        let Some((pos, quote, pat_len)) = next_attr_occurrence(input, cursor, &dq_pat, &sq_pat)
        else {
            out.push_str(&input[cursor..]);
            break;
        };

        out.push_str(&input[cursor..pos + pat_len]);
        let val_start = pos + pat_len;
        let Some(end_rel) = input[val_start..].find(quote) else {
            out.push_str(&input[val_start..]);
            break;
        };
        let val_end = val_start + end_rel;
        out.push_str(&rewrite_html_link_value(
            &input[val_start..val_end],
            prefix,
            force_index,
        ));
        out.push(quote);
        cursor = val_end + quote.len_utf8();
    }

    out
}

fn next_attr_occurrence(
    input: &str,
    start: usize,
    dq_pat: &str,
    sq_pat: &str,
) -> Option<(usize, char, usize)> {
    let dq = input[start..]
        .find(dq_pat)
        .map(|p| (start + p, '"', dq_pat.len()));
    let sq = input[start..]
        .find(sq_pat)
        .map(|p| (start + p, '\'', sq_pat.len()));
    match (dq, sq) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn rewrite_html_link_value(value: &str, prefix: &str, force_index: bool) -> String {
    if !is_rewritable_html_link(value) {
        return value.to_string();
    }

    let cut = value.find(['?', '#']).unwrap_or(value.len());
    let (path_part, suffix) = value.split_at(cut);
    if let Some(path) = path_part.strip_prefix('/') {
        return format!(
            "{prefix}{}{}",
            path_for_file_scheme(path, force_index),
            suffix
        );
    }
    format!("{}{}", path_for_file_scheme(path_part, force_index), suffix)
}

pub(super) fn is_rewritable_html_link(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('#')
        || value.starts_with('?')
        || value.starts_with("//")
        || value.starts_with("data:")
        || value.starts_with("mailto:")
        || value.starts_with("tel:")
        || value.starts_with("javascript:")
    {
        return false;
    }

    if let Some((head, _)) = value.split_once(':')
        && !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return false;
    }
    true
}

pub(super) fn path_for_file_scheme(path: &str, force_index: bool) -> String {
    if path.is_empty() {
        return "index.html".to_string();
    }
    if !force_index {
        return path.to_string();
    }
    if path.ends_with('/') {
        return format!("{path}index.html");
    }

    let last = path.rsplit('/').next().unwrap_or(path);
    if last.contains('.') {
        path.to_string()
    } else {
        format!("{path}/index.html")
    }
}

/// CSS内の `url(/...)` を file:// 向け相対パスへ変換する。
pub(super) fn rewrite_css_root_urls(input: &str, prefix: &str) -> String {
    let mut s = input.to_string();
    s = s.replace("url(\"//", "url(\"__TRACEY_KEEP_URL_DBL_DQ__");
    s = s.replace("url('//", "url('__TRACEY_KEEP_URL_DBL_SQ__");
    s = s.replace("url(//", "url(__TRACEY_KEEP_URL_DBL__)");
    s = s.replace("url('/", &format!("url('{prefix}"));
    s = s.replace("url(\"/", &format!("url(\"{prefix}"));
    s = s.replace("url(/", &format!("url({prefix}"));
    s = s.replace("__TRACEY_KEEP_URL_DBL__", "//");
    s = s.replace("__TRACEY_KEEP_URL_DBL_DQ__", "//");
    s = s.replace("__TRACEY_KEEP_URL_DBL_SQ__", "//");
    s
}

#[cfg(test)]
mod tests {
    use super::{rewrite_css_root_urls, rewrite_html_root_links};

    #[test]
    fn html_root_links_are_rewritten() {
        let html = r#"<a href="/guide/configuration"></a>"#;
        let got = rewrite_html_root_links(html, "../../");
        assert!(got.contains(r#"href="../../guide/configuration/index.html""#));
    }

    #[test]
    fn css_root_urls_are_rewritten() {
        let css = r#"a{background:url(/fonts/a.woff2)}b{background:url("//cdn/x.png")}"#;
        let got = rewrite_css_root_urls(css, "./");
        assert!(got.contains("url(./fonts/a.woff2)"));
        assert!(got.contains(r#"url("//cdn/x.png")"#));
    }
}
