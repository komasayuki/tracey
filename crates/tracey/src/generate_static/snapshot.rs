use std::path::Path;

use eyre::{Result, WrapErr};
use tracey_api::ApiCodeUnit;

use crate::generate_static::bundle::{
    StaticBundle, StaticChunk, StaticEntryChunk, StaticEntryManifest, StaticFileChunk,
    StaticFileRef, StaticHealth, StaticManifest, entry_chunk_path, file_chunk_path,
};
use crate::generate_static::rendering::{arborium_language, display_path, html_escape};
use crate::generate_static::sanitize::{
    sanitize_static_config, sanitize_static_forward, sanitize_static_spec_content,
};

pub(super) async fn build(project_root: &Path) -> Result<StaticBundle> {
    let config_path = project_root.join(".config/tracey/config.styx");
    let (config, mut config_error) = load_config_with_error(&config_path)?;

    let data = match crate::data::build_dashboard_data(project_root, &config, 1, true).await {
        Ok(data) => data,
        Err(e) => {
            let message = format!("Config validation error: {e}");
            config_error = Some(match config_error {
                Some(prev) => format!("{prev}\n\n{message}"),
                None => message,
            });
            crate::data::build_dashboard_data(
                project_root,
                &crate::config::Config::default(),
                1,
                true,
            )
            .await
            .wrap_err("Failed to build fallback dashboard data")?
        }
    };

    let mut manifest_entries = Vec::new();
    let mut entry_chunks = Vec::new();
    let mut file_chunks = Vec::new();
    let mut highlighter = arborium::Highlighter::new();

    for ((spec, impl_name), forward) in &data.forward_by_impl {
        let Some(reverse) = data
            .reverse_by_impl
            .get(&(spec.clone(), impl_name.clone()))
            .cloned()
        else {
            continue;
        };

        let mut forward = forward.clone();
        let mut spec_content =
            render_spec_content(project_root, &data, spec, impl_name, &forward).await?;
        let files = build_file_chunks(project_root, &data, spec, impl_name, &mut highlighter)?;

        sanitize_static_forward(project_root, &mut forward);
        sanitize_static_spec_content(project_root, &mut spec_content);

        let entry_path = entry_chunk_path(spec, impl_name);
        let mut file_refs = Vec::with_capacity(files.len());

        for chunk in files {
            file_refs.push(StaticFileRef {
                path: chunk.data.path.clone(),
                chunk: chunk.relative_path.clone(),
            });
            file_chunks.push(chunk);
        }

        canonicalize_entry(&mut forward, &mut spec_content, &mut file_refs);

        manifest_entries.push(StaticEntryManifest {
            spec: spec.clone(),
            impl_name: impl_name.clone(),
            chunk: entry_path.clone(),
        });
        entry_chunks.push(StaticChunk {
            relative_path: entry_path,
            data: StaticEntryChunk {
                spec: spec.clone(),
                impl_name: impl_name.clone(),
                forward,
                reverse,
                spec_content,
                files: file_refs,
            },
        });
    }

    manifest_entries.sort_by(|a, b| (&a.spec, &a.impl_name).cmp(&(&b.spec, &b.impl_name)));
    entry_chunks.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    file_chunks.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(StaticBundle {
        manifest: StaticManifest {
            version: data.version,
            config: sanitize_static_config(data.config),
            health: StaticHealth { config_error },
            entries: manifest_entries,
        },
        entry_chunks,
        file_chunks,
    })
}

fn canonicalize_entry(
    forward: &mut tracey_api::ApiSpecForward,
    _spec_content: &mut tracey_api::ApiSpecData,
    file_refs: &mut [StaticFileRef],
) {
    forward.rules.sort_by(|a, b| a.id.cmp(&b.id));
    for rule in &mut forward.rules {
        rule.impl_refs.sort_by(ref_sort_key);
        rule.verify_refs.sort_by(ref_sort_key);
        rule.depends_refs.sort_by(ref_sort_key);
        rule.stale_refs.sort_by(|a, b| {
            (&a.file, a.line, &a.reference_id).cmp(&(&b.file, b.line, &b.reference_id))
        });
    }
    file_refs.sort_by(|a, b| a.path.cmp(&b.path));
}

fn ref_sort_key(a: &tracey_api::ApiCodeRef, b: &tracey_api::ApiCodeRef) -> std::cmp::Ordering {
    (&a.file, a.line).cmp(&(&b.file, b.line))
}

fn load_config_with_error(config_path: &Path) -> Result<(crate::config::Config, Option<String>)> {
    if !config_path.exists() {
        return Ok((crate::config::Config::default(), None));
    }

    let content = std::fs::read_to_string(config_path)
        .wrap_err_with(|| format!("Failed to read config {}", config_path.display()))?;

    match facet_styx::from_str::<crate::config::Config>(&content) {
        Ok(cfg) => Ok((cfg, None)),
        Err(e) => Ok((
            crate::config::Config::default(),
            Some(format!(
                "Config file {} has errors:\n{}",
                config_path.display(),
                e
            )),
        )),
    }
}

async fn render_spec_content(
    project_root: &Path,
    data: &crate::data::DashboardData,
    spec: &str,
    impl_name: &str,
    forward: &tracey_api::ApiSpecForward,
) -> Result<tracey_api::ApiSpecData> {
    let include_patterns = data
        .spec_includes_by_name
        .get(spec)
        .cloned()
        .unwrap_or_default();

    crate::data::render_spec_content_for_impl(
        project_root,
        &include_patterns,
        spec,
        impl_name,
        forward,
    )
    .await
    .wrap_err_with(|| format!("Failed to render spec content for {spec}/{impl_name}"))
}

fn build_file_chunks(
    project_root: &Path,
    data: &crate::data::DashboardData,
    spec: &str,
    impl_name: &str,
    highlighter: &mut arborium::Highlighter,
) -> Result<Vec<StaticChunk<StaticFileChunk>>> {
    let Some(code_units_by_file) = data
        .code_units_by_impl
        .get(&(spec.to_string(), impl_name.to_string()))
    else {
        return Ok(Vec::new());
    };

    let mut chunks = Vec::new();
    for (full_path, units) in code_units_by_file {
        let relative = display_path(project_root, full_path);
        let content = match std::fs::read_to_string(full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let html = match arborium_language(&relative) {
            Some(lang) => highlighter
                .highlight(lang, &content)
                .unwrap_or_else(|_| html_escape(&content)),
            None => html_escape(&content),
        };

        let mut api_units: Vec<ApiCodeUnit> = units
            .iter()
            .map(|u| ApiCodeUnit {
                kind: format!("{:?}", u.kind).to_lowercase(),
                name: u.name.clone(),
                start_line: u.start_line,
                end_line: u.end_line,
                rule_refs: u.req_refs.iter().map(|r| r.to_string()).collect(),
            })
            .collect();
        api_units.sort_by(|a, b| {
            (&a.start_line, &a.end_line, &a.kind, &a.name).cmp(&(
                &b.start_line,
                &b.end_line,
                &b.kind,
                &b.name,
            ))
        });

        chunks.push(StaticChunk {
            relative_path: file_chunk_path(spec, impl_name, &relative),
            data: StaticFileChunk {
                path: relative.clone(),
                data: tracey_api::ApiFileData {
                    path: relative,
                    content,
                    html,
                    units: api_units,
                },
            },
        });
    }

    chunks.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(chunks)
}
