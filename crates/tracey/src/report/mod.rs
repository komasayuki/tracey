use std::fs;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr};

mod markdown;
mod meta;

const DEFAULT_OUTPUT_DIR: &str = "docs/traceability_report_txt";
const REPORT_FILE_NAME: &str = "traceability_report.md";

/// `tracey report` の実体。
/// Markdown 形式のトレーサビリティレポートを 1 ファイルで出力する。
pub async fn run(root: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let project_root = root.unwrap_or_else(|| crate::find_project_root().unwrap_or_default());
    let output_dir = resolve_output_dir(&project_root, output);
    let config_path = project_root.join(".config/tracey/config.styx");
    let config = crate::load_config(&config_path)?;
    let data = crate::data::build_dashboard_data(&project_root, &config, 1, true).await?;
    let report_meta = meta::collect(&project_root);
    let report_path = output_dir.join(REPORT_FILE_NAME);

    fs::create_dir_all(&output_dir)
        .wrap_err_with(|| format!("Failed to create output dir {}", output_dir.display()))?;

    let content = markdown::render(&project_root, &report_path, &data, &report_meta);
    fs::write(&report_path, content)
        .wrap_err_with(|| format!("Failed to write report {}", report_path.display()))?;

    println!("Generated markdown report at {}", report_path.display());
    Ok(())
}

fn resolve_output_dir(project_root: &Path, output: Option<PathBuf>) -> PathBuf {
    match output {
        Some(path) if path.is_absolute() => path,
        Some(path) => project_root.join(path),
        None => project_root.join(DEFAULT_OUTPUT_DIR),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::resolve_output_dir;

    #[test]
    fn default_output_is_docs_traceability_report_txt() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, None);
        assert_eq!(
            got,
            PathBuf::from("project-root/docs/traceability_report_txt")
        );
    }

    #[test]
    fn relative_output_is_resolved_from_project_root() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, Some(PathBuf::from("reports/md")));
        assert_eq!(got, PathBuf::from("project-root/reports/md"));
    }
}
