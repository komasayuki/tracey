use std::path::Path;

use tracey_api::{ApiConfig, ApiSpecData, ApiSpecForward};

use crate::generate_static::rendering::display_path;

pub(super) fn sanitize_static_config(mut config: ApiConfig) -> ApiConfig {
    // 静的生成物にローカルの絶対パスを残さない。
    config.project_root.clear();
    config
}

pub(super) fn sanitize_static_forward(project_root: &Path, forward: &mut ApiSpecForward) {
    for rule in &mut forward.rules {
        if let Some(path) = &mut rule.source_file
            && Path::new(path).is_absolute()
        {
            *path = display_path(project_root, Path::new(path));
        }

        // static export では editor 連携を使わないので、差分ノイズになる情報を落とす。
        rule.source_file = None;
        rule.source_line = None;
        rule.source_column = None;
        strip_edit_metadata(&mut rule.html);
    }
}

pub(super) fn sanitize_static_spec_content(project_root: &Path, spec_content: &mut ApiSpecData) {
    let root_prefix = format!("{}/", project_root.display());
    let canonical_prefix = project_root
        .canonicalize()
        .ok()
        .map(|path| format!("{}/", path.display()));

    for section in &mut spec_content.sections {
        if Path::new(&section.source_file).is_absolute() {
            section.source_file = display_path(project_root, Path::new(&section.source_file));
        }
        section.html = section.html.replace(&root_prefix, "");
        if let Some(prefix) = &canonical_prefix {
            section.html = section.html.replace(prefix, "");
        }
        strip_edit_metadata(&mut section.html);
    }
}

fn strip_edit_metadata(html: &mut String) {
    strip_attr(html, "data-source-file");
    strip_attr(html, "data-source-line");
    strip_attr(html, "data-br");
    strip_req_badges_right(html);
}

fn strip_attr(html: &mut String, attr: &str) {
    let pattern = format!(" {attr}=\"");
    while let Some(start) = html.find(&pattern) {
        let value_start = start + pattern.len();
        let Some(end_rel) = html[value_start..].find('"') else {
            break;
        };
        let end = value_start + end_rel + 1;
        html.replace_range(start..end, "");
    }
}

fn strip_req_badges_right(html: &mut String) {
    let marker = r#"<div class="req-badges-right">"#;
    while let Some(start) = html.find(marker) {
        let Some(end_rel) = html[start..].find("</div>") else {
            break;
        };
        let end = start + end_rel + "</div>".len();
        html.replace_range(start..end, "");
    }
}

#[cfg(test)]
mod tests {
    use super::strip_edit_metadata;

    #[test]
    fn strip_edit_metadata_removes_editor_specific_attributes() {
        let mut html = r#"<div data-source-file="spec.md" data-source-line="12" data-br="1-2"><div class="req-badges-right"><button class="req-badge req-edit">Edit</button></div></div>"#.to_string();
        strip_edit_metadata(&mut html);
        assert!(!html.contains("data-source-file"));
        assert!(!html.contains("data-source-line"));
        assert!(!html.contains("data-br"));
        assert!(!html.contains("req-badges-right"));
    }
}
