use std::path::Path;
use std::process::Command;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone)]
pub(super) struct ReportMeta {
    pub generated_at: String,
    pub git_hash: String,
}

/// 実行環境のローカル時刻と git hash を収集する。
pub(super) fn collect(project_root: &Path) -> ReportMeta {
    ReportMeta {
        generated_at: local_timestamp(),
        git_hash: git_hash(project_root),
    }
}

fn local_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let local = now.to_offset(offset);
    local.format(&Rfc3339).unwrap_or_else(|_| local.to_string())
}

fn git_hash(project_root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--short=12", "HEAD"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let text = String::from_utf8_lossy(&result.stdout).trim().to_string();
            if text.is_empty() {
                "-".to_string()
            } else {
                text
            }
        }
        _ => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::collect;

    #[test]
    fn missing_git_repo_returns_dash() {
        let meta = collect(Path::new("/tmp/tracey-report-no-git"));
        assert!(!meta.generated_at.is_empty());
        assert_eq!(meta.git_hash, "-");
    }
}
