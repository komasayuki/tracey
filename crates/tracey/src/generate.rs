use eyre::Result;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT_DIR: &str = "docs/generate";

/// `tracey generate` の実体。
/// `tracey web` と同じダッシュボードを静的サイトとして出力する。
pub async fn run(root: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let project_root = root.unwrap_or_else(|| crate::find_project_root().unwrap_or_default());
    let output_dir = resolve_output_dir(&project_root, output);

    crate::generate_static::generate(&project_root, &output_dir).await?;

    println!("Generated static site at {}", output_dir.display());
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
    use super::resolve_output_dir;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_output_is_docs_generate_under_project_root() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, None);
        assert_eq!(got, PathBuf::from("project-root/docs/generate"));
    }

    #[test]
    fn relative_output_is_resolved_from_project_root() {
        let root = Path::new("project-root");
        let got = resolve_output_dir(root, Some(PathBuf::from("site-out")));
        assert_eq!(got, PathBuf::from("project-root/site-out"));
    }
}
