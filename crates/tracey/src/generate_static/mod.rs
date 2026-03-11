use std::fs;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr, eyre};

mod rendering;
mod shim;
mod snapshot;

const INDEX_CSS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.css");
const INDEX_JS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.js");
const STATIC_DATA_JSON_PATH: &str = "tracey-static/api-data.json";
const STATIC_DATA_JS_PATH: &str = "tracey-static/api-data.js";
const STATIC_RUNTIME_JS_PATH: &str = "tracey-static/runtime.js";
const STATIC_RUNTIME_CSS_PATH: &str = "tracey-static/runtime.css";
const STATIC_HIDE_EDIT_CSS: &str =
    r#".req-badges-right,.req-badge.req-edit{display:none!important;}"#;

pub(crate) async fn generate(project_root: &Path, output_dir: &Path) -> Result<()> {
    let snapshot = snapshot::build(project_root).await?;
    prepare_output_dir(output_dir)?;
    write_static_assets(output_dir, &snapshot)?;
    write_pages(output_dir, &snapshot.config)?;
    Ok(())
}

fn prepare_output_dir(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .wrap_err_with(|| format!("Failed to clean output dir {}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir)
        .wrap_err_with(|| format!("Failed to create output dir {}", output_dir.display()))?;
    Ok(())
}

fn write_static_assets(output_dir: &Path, snapshot: &snapshot::StaticSnapshot) -> Result<()> {
    let static_dir = output_dir.join("tracey-static");
    fs::create_dir_all(&static_dir)
        .wrap_err_with(|| format!("Failed to create static data dir {}", static_dir.display()))?;

    let runtime_js = format!("{}\n{}", shim::runtime_prelude(), dashboard_runtime_js());
    fs::write(static_dir.join("runtime.js"), runtime_js)
        .wrap_err("Failed to write static runtime JS")?;
    fs::write(static_dir.join("runtime.css"), dashboard_runtime_css())
        .wrap_err("Failed to write static runtime CSS")?;

    let json =
        facet_json::to_string_pretty(snapshot).map_err(|e| eyre!("JSON serialize failed: {e}"))?;
    fs::write(static_dir.join("api-data.json"), &json)
        .wrap_err("Failed to write static API data")?;
    fs::write(
        static_dir.join("api-data.js"),
        format!("window.__TRACEY_STATIC_SNAPSHOT__ = {json};\n"),
    )
    .wrap_err("Failed to write static API script")?;

    Ok(())
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
    format!("{INDEX_CSS}\n{STATIC_HIDE_EDIT_CSS}\n")
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
    let css_path = format!("{prefix}{STATIC_RUNTIME_CSS_PATH}");
    let data_path = format!("{prefix}{STATIC_DATA_JSON_PATH}");
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
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Recursive:slnt,wght,CASL,CRSV,MONO@-15..0,300..1000,0..1,0..1,0..1&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@arborium/arborium@2.4.6/dist/themes/base.css">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@arborium/arborium@2.4.6/dist/themes/kanagawa-dragon.css">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@arborium/arborium@2.4.6/dist/themes/github-light.css">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/devicon.min.css">
  <link rel="stylesheet" href="{css_path}">
  <script src="https://cdn.jsdelivr.net/npm/lucide@0.469.0/dist/umd/lucide.min.js"></script>
  <script>
    window.__TRACEY_STATIC_BOOTSTRAP__ = {{
      routePath: "{route_path}",
      dataPath: "{data_path}"
    }};
  </script>
  <script src="{data_js_path}"></script>
  <script defer src="{runtime_js_path}"></script>
</head>
<body>
  <div id="app"><div class="loading">Loading...</div></div>
</body>
</html>
"#
    )
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
    use super::{dashboard_runtime_css, render_page_html};

    #[test]
    fn static_output_hides_edit_button() {
        let css = dashboard_runtime_css();
        assert!(css.contains(".req-badges-right,.req-badge.req-edit{display:none!important;}"));
    }

    #[test]
    fn static_output_uses_shared_runtime_assets() {
        let html = render_page_html("./", "/tracey/main/spec");
        assert!(html.contains("./tracey-static/runtime.css"));
        assert!(html.contains("./tracey-static/runtime.js"));
        assert!(html.contains("window.__TRACEY_STATIC_BOOTSTRAP__"));
    }
}
