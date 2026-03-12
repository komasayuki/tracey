use std::fs;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr, eyre};

use crate::generate_static::bundle::{StaticBundle, StaticChunk};
use crate::generate_static::shim;
use crate::generate_static::vendor_assets::VENDOR_ASSETS;

const INDEX_CSS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.css");
const INDEX_JS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.js");
const STATIC_DATA_JS_PATH: &str = "tracey-static/api-data.js";
const STATIC_RUNTIME_JS_PATH: &str = "tracey-static/runtime.js";
const STATIC_RUNTIME_CSS_PATH: &str = "tracey-static/runtime.css";
const STATIC_HIDE_UI_CSS: &str = r#"
.req-badges-right,
.req-badge.req-edit {
  display: none !important;
}

.tracey-static-route-sources .stats-bar,
.tracey-static-route-sources .tree-file-badge {
  display: none !important;
}
"#;

pub(super) fn write_site(output_dir: &Path, bundle: &StaticBundle) -> Result<()> {
    write_static_assets(output_dir, bundle)?;
    write_pages(output_dir, &bundle.manifest.config)?;
    Ok(())
}

fn write_static_assets(output_dir: &Path, bundle: &StaticBundle) -> Result<()> {
    let static_dir = output_dir.join("tracey-static");
    fs::create_dir_all(&static_dir)
        .wrap_err_with(|| format!("Failed to create static data dir {}", static_dir.display()))?;

    let runtime_js = format!("{}\n{}", shim::runtime_prelude(), dashboard_runtime_js());
    fs::write(static_dir.join("runtime.js"), runtime_js)
        .wrap_err("Failed to write static runtime JS")?;
    fs::write(static_dir.join("runtime.css"), dashboard_runtime_css())
        .wrap_err("Failed to write static runtime CSS")?;
    fs::write(
        static_dir.join("api-data.js"),
        js_assignment("__TRACEY_STATIC_MANIFEST__", &bundle.manifest)?,
    )
    .wrap_err("Failed to write static manifest script")?;

    for chunk in &bundle.entry_chunks {
        write_chunk_script(&static_dir, chunk)?;
    }
    for chunk in &bundle.file_chunks {
        write_chunk_script(&static_dir, chunk)?;
    }
    for asset in VENDOR_ASSETS {
        let path = static_dir.join(asset.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("Failed to create asset dir {}", parent.display()))?;
        }
        fs::write(&path, asset.bytes)
            .wrap_err_with(|| format!("Failed to write asset {}", path.display()))?;
    }

    Ok(())
}

fn write_chunk_script<T>(static_dir: &Path, chunk: &StaticChunk<T>) -> Result<()>
where
    T: for<'a> facet::Facet<'a>,
{
    let path = static_dir.join(&chunk.relative_path);
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("Chunk path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .wrap_err_with(|| format!("Failed to create chunk dir {}", parent.display()))?;
    fs::write(
        &path,
        js_chunk_assignment(&chunk.relative_path, &chunk.data)
            .wrap_err("Chunk serialize failed")?,
    )
    .wrap_err_with(|| format!("Failed to write chunk {}", path.display()))?;
    Ok(())
}

fn js_assignment<T>(name: &str, value: &T) -> Result<String>
where
    T: for<'a> facet::Facet<'a>,
{
    let json =
        facet_json::to_string_pretty(value).map_err(|e| eyre!("JSON serialize failed: {e}"))?;
    Ok(format!("window.{name} = {json};\n"))
}

fn js_chunk_assignment<T>(path: &str, value: &T) -> Result<String>
where
    T: for<'a> facet::Facet<'a>,
{
    let json =
        facet_json::to_string_pretty(value).map_err(|e| eyre!("JSON serialize failed: {e}"))?;
    Ok(format!(
        "window.__TRACEY_STATIC_CHUNKS__ = window.__TRACEY_STATIC_CHUNKS__ || {{}};\nwindow.__TRACEY_STATIC_CHUNKS__[\"{path}\"] = {json};\n"
    ))
}

fn dashboard_runtime_js() -> String {
    let runtime = INDEX_JS
        .replace(
            "window.location.pathname",
            "window.__TRACEY_EFFECTIVE_PATHNAME__()",
        )
        .replace(
            "location.pathname",
            "window.__TRACEY_EFFECTIVE_PATHNAME__()",
        );
    format!("(function() {{\n{runtime}\n}})();\n")
}

fn dashboard_runtime_css() -> String {
    format!("{INDEX_CSS}\n{STATIC_HIDE_UI_CSS}\n")
}

fn write_pages(output_dir: &Path, config: &tracey_api::ApiConfig) -> Result<()> {
    let root_route = default_route_path(config);
    write_page(
        output_dir.join("index.html"),
        &render_page_html("./", &root_route),
    )?;

    for route in build_routes(config) {
        let depth = route.components().count();
        let prefix = if depth == 0 {
            "./".to_string()
        } else {
            "../".repeat(depth)
        };
        let route_path = format!("/{}", route.to_string_lossy().replace('\\', "/"));
        write_page(
            output_dir.join(route).join("index.html"),
            &render_page_html(&prefix, &route_path),
        )?;
    }

    Ok(())
}

fn render_page_html(prefix: &str, route_path: &str) -> String {
    let body_class = route_body_class(route_path);
    let static_root = format!("{prefix}tracey-static/");
    let css_path = format!("{prefix}{STATIC_RUNTIME_CSS_PATH}");
    let data_js_path = format!("{prefix}{STATIC_DATA_JS_PATH}");
    let runtime_js_path = format!("{prefix}{STATIC_RUNTIME_JS_PATH}");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>tracey</title>
  <link rel="icon" type="image/svg+xml" href="data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHJlY3Qgd2lkdGg9IjMyIiBoZWlnaHQ9IjMyIiByeD0iNiIgZmlsbD0iIzFhMWIyNiIvPjxwYXRoIGQ9Ik04IDI0IEwxNCA4IEwxOCAxNiBMMjQgOCIgc3Ryb2tlPSIjN2FhMmY3IiBzdHJva2Utd2lkdGg9IjMiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIgZmlsbD0ibm9uZSIvPjxjaXJjbGUgY3g9IjgiIGN5PSIyNCIgcj0iMi41IiBmaWxsPSIjNzNkYWNhIi8+PGNpcmNsZSBjeD0iMjQiIGN5PSI4IiByPSIyLjUiIGZpbGw9IiM3M2RhY2EiLz48L3N2Zz4=">
  <link rel="stylesheet" href="{prefix}tracey-static/vendor/recursive/recursive.css">
  <link rel="stylesheet" href="{prefix}tracey-static/vendor/arborium/base.css">
  <link rel="stylesheet" href="{prefix}tracey-static/vendor/arborium/kanagawa-dragon.css">
  <link rel="stylesheet" href="{prefix}tracey-static/vendor/arborium/github-light.css">
  <link rel="stylesheet" href="{prefix}tracey-static/vendor/devicon/devicon.min.css">
  <link rel="stylesheet" href="{css_path}">
  <script src="{prefix}tracey-static/vendor/lucide/lucide.min.js"></script>
  <script>
    window.__TRACEY_STATIC_BOOTSTRAP__ = {{
      routePath: "{route_path}",
      staticRoot: "{static_root}"
    }};
  </script>
  <script src="{data_js_path}"></script>
  <script defer src="{runtime_js_path}"></script>
</head>
<body class="{body_class}">
  <div id="app"><div class="loading">Loading...</div></div>
</body>
</html>
"#
    )
}

fn route_body_class(route_path: &str) -> &'static str {
    if route_path.ends_with("/sources") {
        "tracey-static-route-sources"
    } else if route_path.ends_with("/coverage") {
        "tracey-static-route-coverage"
    } else if route_path.ends_with("/spec") {
        "tracey-static-route-spec"
    } else {
        "tracey-static-route-default"
    }
}

fn default_route_path(config: &tracey_api::ApiConfig) -> String {
    let Some(spec) = config.specs.first() else {
        return "/".to_string();
    };
    let Some(impl_name) = spec.implementations.first() else {
        return "/".to_string();
    };
    let spec_enc = urlencoding::encode(&spec.name);
    let impl_enc = urlencoding::encode(impl_name);
    format!("/{spec_enc}/{impl_enc}/spec")
}

fn write_page(path: PathBuf, html: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("Page path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .wrap_err_with(|| format!("Failed to create page dir {}", parent.display()))?;
    fs::write(&path, html).wrap_err_with(|| format!("Failed to write page {}", path.display()))?;
    Ok(())
}

fn build_routes(config: &tracey_api::ApiConfig) -> Vec<PathBuf> {
    let mut routes = Vec::new();
    for spec in &config.specs {
        let spec_enc = urlencoding::encode(&spec.name).into_owned();
        routes.push(PathBuf::from(&spec_enc));
        for impl_name in &spec.implementations {
            let impl_enc = urlencoding::encode(impl_name).into_owned();
            let base = PathBuf::from(&spec_enc).join(&impl_enc);
            routes.push(base.clone());
            routes.push(base.join("spec"));
            routes.push(base.join("coverage"));
            routes.push(base.join("sources"));
        }
    }
    routes
}

#[cfg(test)]
mod tests {
    use super::{dashboard_runtime_css, render_page_html, route_body_class};

    #[test]
    fn static_output_uses_local_vendor_assets() {
        let html = render_page_html("./", "/tracey/main/spec");
        assert!(html.contains("./tracey-static/vendor/recursive/recursive.css"));
        assert!(html.contains("./tracey-static/vendor/lucide/lucide.min.js"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("cdn.jsdelivr.net"));
    }

    #[test]
    fn static_output_uses_shared_runtime_assets() {
        let html = render_page_html("./", "/tracey/main/spec");
        assert!(html.contains("./tracey-static/runtime.css"));
        assert!(html.contains("./tracey-static/runtime.js"));
        assert!(html.contains("./tracey-static/api-data.js"));
        assert!(html.contains("window.__TRACEY_STATIC_BOOTSTRAP__"));
        assert!(html.contains(r#"class="tracey-static-route-spec""#));
        assert!(!html.contains("api-data.json"));
    }

    #[test]
    fn static_output_hides_sources_page_stats() {
        let css = dashboard_runtime_css();
        assert!(css.contains(".tracey-static-route-sources .stats-bar"));
        assert!(css.contains(".tracey-static-route-sources .tree-file-badge"));
    }

    #[test]
    fn route_body_class_changes_by_view() {
        assert_eq!(
            route_body_class("/tracey/main/sources"),
            "tracey-static-route-sources"
        );
        assert_eq!(
            route_body_class("/tracey/main/coverage"),
            "tracey-static-route-coverage"
        );
        assert_eq!(
            route_body_class("/tracey/main/spec"),
            "tracey-static-route-spec"
        );
    }
}
