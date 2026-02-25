use eyre::{Result, WrapErr};
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod link_rewrite;
mod title_fill;

/// 生成HTML/CSSを file:// で閲覧しやすい形に整える。
pub(crate) fn rewrite_for_file_scheme(project_root: &Path, output_dir: &Path) -> Result<usize> {
    let mut rewritten = 0usize;
    let mut title_cache: HashMap<PathBuf, Option<String>> = HashMap::new();
    let content_dir = resolve_content_dir(project_root);
    let walker = WalkBuilder::new(output_dir)
        .hidden(false)
        .git_ignore(false)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext != "html" && ext != "css" {
            continue;
        }

        let dir = path.parent().unwrap_or(output_dir);
        let prefix = relative_prefix(output_dir, dir);
        let original = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read generated file {}", path.display()))?;
        let updated = if ext == "html" {
            let relinked = link_rewrite::rewrite_html_root_links(&original, &prefix);
            title_fill::fill_empty_anchor_titles(
                &relinked,
                path,
                output_dir,
                &content_dir,
                &mut title_cache,
            )
        } else {
            link_rewrite::rewrite_css_root_urls(&original, &prefix)
        };

        if updated != original {
            std::fs::write(path, updated)
                .wrap_err_with(|| format!("Failed to write generated file {}", path.display()))?;
            rewritten += 1;
        }
    }

    Ok(rewritten)
}

fn resolve_content_dir(project_root: &Path) -> PathBuf {
    let config_path = project_root.join(".config/dodeca.styx");
    if let Ok(content) = std::fs::read_to_string(&config_path)
        && let Some(path) = parse_content_path_from_styx(&content)
    {
        return project_root.join(path);
    }

    let docs_content = project_root.join("docs/content");
    if docs_content.is_dir() {
        docs_content
    } else {
        project_root.join("content")
    }
}

fn parse_content_path_from_styx(content: &str) -> Option<String> {
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(rest) = t.strip_prefix("content ") {
            let token = rest.split_whitespace().next()?;
            return Some(token.trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

fn relative_prefix(root: &Path, dir: &Path) -> String {
    let depth = dir
        .strip_prefix(root)
        .ok()
        .map(|p| p.components().count())
        .unwrap_or(0);
    if depth == 0 {
        "./".to_string()
    } else {
        "../".repeat(depth)
    }
}
