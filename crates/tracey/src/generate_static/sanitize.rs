use std::path::Path;

use tracey_api::{ApiConfig, ApiSpecData, ApiSpecForward};

use crate::generate_static::rendering::display_path;

pub(super) fn sanitize_static_config(mut config: ApiConfig) -> ApiConfig {
    // 静的生成物にローカルの絶対パスを残さない。
    config.project_root.clear();
    config
}

pub(super) fn sanitize_static_paths(
    project_root: &Path,
    forward: &mut ApiSpecForward,
    spec_content: &mut ApiSpecData,
) {
    let root_prefix = format!("{}/", project_root.display());
    let canonical_prefix = project_root
        .canonicalize()
        .ok()
        .map(|path| format!("{}/", path.display()));

    for rule in &mut forward.rules {
        if let Some(path) = &mut rule.source_file
            && Path::new(path).is_absolute()
        {
            *path = display_path(project_root, Path::new(path));
        }
    }

    for section in &mut spec_content.sections {
        if Path::new(&section.source_file).is_absolute() {
            section.source_file = display_path(project_root, Path::new(&section.source_file));
        }
        section.html = section.html.replace(&root_prefix, "");
        if let Some(prefix) = &canonical_prefix {
            section.html = section.html.replace(prefix, "");
        }
    }
}
