use std::collections::BTreeMap;
use std::path::Path;

use eyre::{Result, WrapErr};
use facet::Facet;
use tracey_api::{
    ApiCodeUnit, ApiConfig, ApiFileData, ApiReverseData, ApiSpecData, ApiSpecForward,
};

use crate::generate_static::rendering::{arborium_language, display_path, html_escape};
use crate::generate_static::sanitize::{sanitize_static_config, sanitize_static_paths};

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticSnapshot {
    pub version: u64,
    pub config: ApiConfig,
    pub health: StaticHealth,
    pub entries: Vec<ImplSnapshot>,
    pub search_rules: Vec<SearchRule>,
    pub search_sources: Vec<SearchSource>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct StaticHealth {
    #[facet(rename = "configError")]
    pub config_error: Option<String>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct ImplSnapshot {
    pub spec: String,
    #[facet(rename = "impl")]
    pub impl_name: String,
    pub forward: ApiSpecForward,
    pub reverse: ApiReverseData,
    pub spec_content: ApiSpecData,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct FileEntry {
    pub path: String,
    pub data: ApiFileData,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct SearchRule {
    pub id: String,
    pub raw: String,
}

#[derive(Debug, Clone, Facet)]
pub(super) struct SearchSource {
    pub path: String,
    pub content: String,
}

pub(super) async fn build(project_root: &Path) -> Result<StaticSnapshot> {
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

    // 画面内検索用に重複を除いたルール/ソース一覧を作る。
    let mut search_rules: BTreeMap<String, SearchRule> = BTreeMap::new();
    let mut search_sources: BTreeMap<String, SearchSource> = BTreeMap::new();
    let mut entries = Vec::new();
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
        sanitize_static_paths(project_root, &mut forward, &mut spec_content);
        let files = build_file_entries(project_root, &data, spec, impl_name, &mut highlighter)?;

        for rule in &forward.rules {
            search_rules
                .entry(rule.id.to_string())
                .or_insert(SearchRule {
                    id: rule.id.to_string(),
                    raw: rule.raw.clone(),
                });
        }

        for file in &files {
            search_sources
                .entry(file.path.clone())
                .or_insert_with(|| SearchSource {
                    path: file.path.clone(),
                    content: file.data.content.clone(),
                });
        }

        entries.push(ImplSnapshot {
            spec: spec.clone(),
            impl_name: impl_name.clone(),
            forward,
            reverse,
            spec_content,
            files,
        });
    }

    entries.sort_by(|a, b| (&a.spec, &a.impl_name).cmp(&(&b.spec, &b.impl_name)));

    let mut snapshot = StaticSnapshot {
        version: data.version,
        config: sanitize_static_config(data.config),
        health: StaticHealth { config_error },
        entries,
        search_rules: search_rules.into_values().collect(),
        search_sources: search_sources.into_values().collect(),
    };
    canonicalize_snapshot(&mut snapshot);
    Ok(snapshot)
}

fn canonicalize_snapshot(snapshot: &mut StaticSnapshot) {
    for entry in &mut snapshot.entries {
        entry.forward.rules.sort_by(|a, b| a.id.cmp(&b.id));
        for rule in &mut entry.forward.rules {
            rule.impl_refs.sort_by(ref_sort_key);
            rule.verify_refs.sort_by(ref_sort_key);
            rule.depends_refs.sort_by(ref_sort_key);
            rule.stale_refs.sort_by(|a, b| {
                (&a.file, a.line, &a.reference_id).cmp(&(&b.file, b.line, &b.reference_id))
            });
        }

        entry.reverse.files.sort_by(|a, b| a.path.cmp(&b.path));
        for file in &mut entry.files {
            file.data.units.sort_by(|a, b| {
                (&a.start_line, &a.end_line, &a.kind, &a.name).cmp(&(
                    &b.start_line,
                    &b.end_line,
                    &b.kind,
                    &b.name,
                ))
            });
        }
        entry.files.sort_by(|a, b| a.path.cmp(&b.path));
    }

    snapshot.search_rules.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.search_sources.sort_by(|a, b| a.path.cmp(&b.path));
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
    forward: &ApiSpecForward,
) -> Result<ApiSpecData> {
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

fn build_file_entries(
    project_root: &Path,
    data: &crate::data::DashboardData,
    spec: &str,
    impl_name: &str,
    highlighter: &mut arborium::Highlighter,
) -> Result<Vec<FileEntry>> {
    let Some(code_units_by_file) = data
        .code_units_by_impl
        .get(&(spec.to_string(), impl_name.to_string()))
    else {
        return Ok(Vec::new());
    };

    let mut entries = Vec::new();
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

        let api_units: Vec<ApiCodeUnit> = units
            .iter()
            .map(|u| ApiCodeUnit {
                kind: format!("{:?}", u.kind).to_lowercase(),
                name: u.name.clone(),
                start_line: u.start_line,
                end_line: u.end_line,
                rule_refs: u.req_refs.iter().map(|r| r.to_string()).collect(),
            })
            .collect();

        entries.push(FileEntry {
            path: relative.clone(),
            data: ApiFileData {
                path: relative,
                content,
                html,
                units: api_units,
            },
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}
