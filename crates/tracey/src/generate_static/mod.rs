use std::fs;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr, eyre};

mod rendering;
mod shim;
mod snapshot;

const INDEX_HTML: &str = include_str!("../bridge/http/dashboard/dist/index.html");
const INDEX_CSS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.css");
const INDEX_JS: &str = include_str!("../bridge/http/dashboard/dist/assets/index.js");
const STATIC_DATA_JSON_PATH: &str = "tracey-static/api-data.json";
const STATIC_DATA_JS_PATH: &str = "tracey-static/api-data.js";

pub(crate) async fn generate(project_root: &Path, output_dir: &Path) -> Result<()> {
    let snapshot = snapshot::build(project_root).await?;
    prepare_output_dir(output_dir)?;
    write_assets(output_dir)?;
    write_snapshot(output_dir, &snapshot)?;
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

fn write_assets(output_dir: &Path) -> Result<()> {
    let assets_dir = output_dir.join("assets");
    fs::create_dir_all(&assets_dir)
        .wrap_err_with(|| format!("Failed to create assets dir {}", assets_dir.display()))?;

    fs::write(assets_dir.join("index.css"), INDEX_CSS).wrap_err("Failed to write dashboard CSS")?;
    fs::write(assets_dir.join("index.js"), INDEX_JS).wrap_err("Failed to write dashboard JS")?;

    Ok(())
}

fn write_snapshot(output_dir: &Path, snapshot: &snapshot::StaticSnapshot) -> Result<()> {
    let static_dir = output_dir.join("tracey-static");
    fs::create_dir_all(&static_dir)
        .wrap_err_with(|| format!("Failed to create static data dir {}", static_dir.display()))?;

    let json = facet_json::to_string(snapshot).map_err(|e| eyre!("JSON serialize failed: {e}"))?;
    fs::write(static_dir.join("api-data.json"), &json)
        .wrap_err("Failed to write static API data")?;
    // file:// 直開きでも確実に読めるよう、JSとしても同梱する。
    fs::write(
        static_dir.join("api-data.js"),
        format!("window.__TRACEY_STATIC_SNAPSHOT__ = {json};\n"),
    )
    .wrap_err("Failed to write static API script")?;

    Ok(())
}

fn write_pages(output_dir: &Path, config: &tracey_api::ApiConfig) -> Result<()> {
    let root_route = default_route_path(config);
    write_page(
        output_dir.join("index.html"),
        &render_page_html("./", &root_route)?,
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
            &render_page_html(&prefix, &route_path)?,
        )?;
    }

    Ok(())
}

fn render_page_html(prefix: &str, route_path: &str) -> Result<String> {
    let static_data_json_path = format!("{prefix}{STATIC_DATA_JSON_PATH}");
    let static_data_js_path = format!("{prefix}{STATIC_DATA_JS_PATH}");
    let injected = shim::inject_static_shim(
        INDEX_HTML,
        &static_data_json_path,
        &static_data_js_path,
        route_path,
    )?;
    Ok(inline_dashboard_assets(&injected))
}

fn inline_dashboard_assets(input: &str) -> String {
    let css = INDEX_CSS.replace("</style>", "<\\/style>");
    let js = INDEX_JS
        .replace(
            "window.location.pathname",
            "window.__TRACEY_EFFECTIVE_PATHNAME__()",
        )
        .replace(
            "location.pathname",
            "window.__TRACEY_EFFECTIVE_PATHNAME__()",
        )
        .replace("</script>", "<\\/script>");
    input
        .replace(
            "<link rel=\"stylesheet\" crossorigin href=\"/assets/index.css\">",
            &format!("<style>\n{css}\n</style>"),
        )
        .replace(
            "<script type=\"module\" crossorigin src=\"/assets/index.js\"></script>",
            &format!("<script type=\"module\">\n{js}\n</script>"),
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
