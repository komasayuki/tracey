use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use super::link_rewrite::{is_rewritable_html_link, path_for_file_scheme};

/// 空テキストの `<a ...></a>` に対してタイトル補完を行う。
pub(super) fn fill_empty_anchor_titles(
    html: &str,
    page_path: &Path,
    output_dir: &Path,
    content_dir: &Path,
    title_cache: &mut HashMap<PathBuf, Option<String>>,
) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    let mut cursor = 0usize;

    while let Some(a_rel) = html[cursor..].find("<a") {
        let a_start = cursor + a_rel;
        out.push_str(&html[cursor..a_start]);
        let Some(tag_end_rel) = html[a_start..].find('>') else {
            out.push_str(&html[a_start..]);
            return out;
        };
        let tag_end = a_start + tag_end_rel;
        let Some(close_rel) = html[tag_end + 1..].find("</a>") else {
            out.push_str(&html[a_start..]);
            return out;
        };
        let close_start = tag_end + 1 + close_rel;
        let tag = &html[a_start..=tag_end];
        let inner = &html[tag_end + 1..close_start];

        if inner.trim().is_empty()
            && let Some(href) = extract_attr_value(tag, "href")
            && let Some(target) = resolve_target_html(page_path, output_dir, &href)
            && let Some(title) = resolve_title_from_target(&target, content_dir, title_cache)
        {
            out.push_str(tag);
            out.push_str(&escape_html_text(&title));
            out.push_str("</a>");
            cursor = close_start + "</a>".len();
            continue;
        }

        out.push_str(&html[a_start..close_start + "</a>".len()]);
        cursor = close_start + "</a>".len();
    }

    out.push_str(&html[cursor..]);
    fill_plain_empty_anchors(&out, page_path, output_dir, content_dir, title_cache)
}

fn extract_attr_value(tag: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pat = format!("{attr}={quote}");
        if let Some(pos) = tag.find(&pat) {
            let start = pos + pat.len();
            let end = tag[start..].find(quote)? + start;
            return Some(tag[start..end].to_string());
        }
    }
    None
}

fn resolve_target_html(page_path: &Path, output_dir: &Path, href: &str) -> Option<PathBuf> {
    if !is_rewritable_html_link(href) {
        return None;
    }
    let cut = href.find(['?', '#']).unwrap_or(href.len());
    let mut path = href[..cut].to_string();
    if path.is_empty() {
        return None;
    }

    path = path_for_file_scheme(&path, true);
    let base = page_path.parent().unwrap_or(output_dir);
    let candidate = if let Some(stripped) = path.strip_prefix('/') {
        output_dir.join(stripped)
    } else {
        base.join(path)
    };
    Some(normalize_path(candidate))
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            _ => out.push(c.as_os_str()),
        }
    }
    out
}

fn resolve_title_from_target(
    target_html: &Path,
    content_dir: &Path,
    title_cache: &mut HashMap<PathBuf, Option<String>>,
) -> Option<String> {
    if let Some(cached) = title_cache.get(target_html) {
        return cached.clone();
    }
    let title = extract_source_rel_path(target_html)
        .and_then(|rel| title_from_markdown_file(&content_dir.join(rel)))
        .or_else(|| title_from_first_h1(target_html));
    title_cache.insert(target_html.to_path_buf(), title.clone());
    title
}

fn extract_source_rel_path(target_html: &Path) -> Option<String> {
    let html = std::fs::read_to_string(target_html).ok()?;
    let pat = r#"data-source-file=""#;
    let start = html.find(pat)? + pat.len();
    let end = html[start..].find('"')? + start;
    Some(html[start..end].to_string())
}

fn title_from_markdown_file(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let mut first_line = lines.next().unwrap_or("");

    if first_line.trim() == "+++" {
        let mut title: Option<String> = None;
        for line in &mut lines {
            if line.trim() == "+++" {
                break;
            }
            if let Some(v) = parse_frontmatter_title(line) {
                title = Some(v);
            }
        }
        if let Some(v) = title {
            return Some(v);
        }
        first_line = lines.next().unwrap_or("");
    }

    // title未指定時は先頭行から `#*\s*` 相当を除去して採用する。
    let t = first_line.trim_start().trim_start_matches('#').trim_start();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_frontmatter_title(line: &str) -> Option<String> {
    let t = line.trim();
    let rest = t.strip_prefix("title")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let value = rest.trim_matches('"').trim_matches('\'').trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn title_from_first_h1(target_html: &Path) -> Option<String> {
    let html = std::fs::read_to_string(target_html).ok()?;
    let h1_start = html.find("<h1")?;
    let after_tag = h1_start + html[h1_start..].find('>')? + 1;
    let close = after_tag + html[after_tag..].find("</h1>")?;
    let text = html[after_tag..close].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn escape_html_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn fill_plain_empty_anchors(
    html: &str,
    page_path: &Path,
    output_dir: &Path,
    content_dir: &Path,
    title_cache: &mut HashMap<PathBuf, Option<String>>,
) -> String {
    let mut out = html.to_string();
    for quote in ['"', '\''] {
        let open = format!("<a href={quote}");
        let close = format!("{quote}></a>");
        let mut cursor = 0usize;
        let mut next = String::with_capacity(out.len() + 64);
        while let Some(rel) = out[cursor..].find(&open) {
            let start = cursor + rel;
            next.push_str(&out[cursor..start]);
            let href_start = start + open.len();
            let Some(end_rel) = out[href_start..].find(quote) else {
                next.push_str(&out[start..]);
                cursor = out.len();
                break;
            };
            let href_end = href_start + end_rel;
            let after = href_end;
            if out[after..].starts_with(&close) {
                let href = &out[href_start..href_end];
                if let Some(target) = resolve_target_html(page_path, output_dir, href)
                    && let Some(title) =
                        resolve_title_from_target(&target, content_dir, title_cache)
                {
                    next.push_str(&format!(
                        "<a href={quote}{href}{quote}>{}</a>",
                        escape_html_text(&title)
                    ));
                    cursor = after + close.len();
                    continue;
                }
            }
            next.push_str(&out[start..href_start]);
            cursor = href_start;
        }
        if cursor < out.len() {
            next.push_str(&out[cursor..]);
        }
        out = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parse_frontmatter_title, title_from_markdown_file};

    #[test]
    fn parse_title_from_markdown_first_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.md");
        std::fs::write(&path, "# Patch Requirements\nbody").expect("write");
        let title = title_from_markdown_file(&path);
        assert_eq!(title.as_deref(), Some("Patch Requirements"));
    }

    #[test]
    fn parse_title_from_frontmatter_title() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.md");
        std::fs::write(
            &path,
            "+++\ntitle = \"Tracey\"\ndescription = \"x\"\n+++\n# ignored",
        )
        .expect("write");
        let title = title_from_markdown_file(&path);
        assert_eq!(title.as_deref(), Some("Tracey"));
        assert_eq!(
            parse_frontmatter_title("title = \"Spec coverage tooling\"").as_deref(),
            Some("Spec coverage tooling")
        );
    }
}
