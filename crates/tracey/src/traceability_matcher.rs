use std::path::{Path, PathBuf};

use tracey_core::is_supported_extension;

#[derive(Debug, Clone)]
pub(crate) struct TraceabilityMatcher {
    roots: Vec<ScanRootPattern>,
    exclude: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScanRootPattern {
    root: PathBuf,
    pattern: String,
    allow_unsupported: bool,
}

impl TraceabilityMatcher {
    pub(crate) fn new(
        project_root: &Path,
        include: &[String],
        exclude: &[String],
    ) -> (Self, Vec<String>) {
        let (roots, warnings) = build_roots(project_root, include);
        (
            Self {
                roots,
                exclude: exclude.to_vec(),
            },
            warnings,
        )
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        let matched_roots: Vec<&ScanRootPattern> = self
            .roots
            .iter()
            .filter(|root| matches_root_pattern(path, root))
            .collect();
        if matched_roots.is_empty() || self.is_excluded(path) {
            return false;
        }
        if path.extension().is_some_and(is_supported_extension) {
            return true;
        }
        matched_roots.iter().any(|root| root.allow_unsupported)
    }

    pub(crate) fn collect_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for root in &self.roots {
            let walker = ignore::WalkBuilder::new(&root.root)
                .follow_links(true)
                .hidden(false)
                .git_ignore(true)
                .build();
            for entry in walker.flatten() {
                let path = entry.path();
                let Some(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_file() && self.matches(path) {
                    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                    files.push(canonical);
                }
            }
        }
        files.sort();
        files.dedup();
        files
    }

    fn is_excluded(&self, path: &Path) -> bool {
        self.roots.iter().any(|root| {
            let Ok(relative) = path.strip_prefix(&root.root) else {
                return false;
            };
            let relative_str = relative.to_string_lossy();
            self.exclude
                .iter()
                .any(|pattern| glob_match(&relative_str, pattern))
        })
    }
}

fn build_roots(project_root: &Path, include: &[String]) -> (Vec<ScanRootPattern>, Vec<String>) {
    let mut roots = Vec::new();
    let mut warnings = Vec::new();

    for pattern in include {
        if pattern.starts_with("../") {
            let base_path = pattern_prefix_before_glob(pattern);
            let resolved_path = project_root.join(&base_path);
            if !resolved_path.exists() {
                warnings.push(format!(
                    "Warning: Cross-workspace path not found: {}\n  Pattern: {}",
                    base_path, pattern
                ));
                continue;
            }
            let adjusted_pattern = pattern
                .strip_prefix(&base_path)
                .unwrap_or(pattern)
                .trim_start_matches('/')
                .to_string();
            roots.push(ScanRootPattern {
                root: resolved_path,
                pattern: adjusted_pattern.clone(),
                allow_unsupported: allows_unsupported_matches(&adjusted_pattern),
            });
            continue;
        }

        roots.push(ScanRootPattern {
            root: project_root.to_path_buf(),
            pattern: pattern.clone(),
            allow_unsupported: allows_unsupported_matches(pattern),
        });
    }

    (roots, warnings)
}

fn pattern_prefix_before_glob(pattern: &str) -> String {
    pattern[..pattern
        .find("**")
        .or_else(|| pattern.find('*'))
        .unwrap_or(pattern.len())]
        .trim_end_matches('/')
        .to_string()
}

fn matches_root_pattern(path: &Path, root: &ScanRootPattern) -> bool {
    let Ok(relative) = path.strip_prefix(&root.root) else {
        return false;
    };
    let relative_str = relative.to_string_lossy();
    glob_match(&relative_str, &root.pattern)
}

fn allows_unsupported_matches(pattern: &str) -> bool {
    let normalized = pattern.replace('\\', "/");
    if !normalized.contains(['*', '?', '[']) {
        return true;
    }
    let Some(last_segment) = normalized.rsplit('/').next() else {
        return false;
    };
    let Some((_, extension)) = last_segment.rsplit_once('.') else {
        return false;
    };
    !extension.contains(['*', '?', '[']) && !extension.is_empty()
}

pub(crate) fn glob_match(path: &str, pattern: &str) -> bool {
    let path = path.replace('\\', "/");
    let pattern = pattern.replace('\\', "/");

    if let Some(ext) = pattern.strip_prefix("**/*.") {
        return path.ends_with(&format!(".{}", ext));
    }
    if let Some(rest) = pattern.strip_prefix("**/") {
        return glob_match(&path, rest);
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix) || path.starts_with(&format!("{}/", prefix));
    }
    if let Some((prefix, suffix)) = pattern.split_once("/**/") {
        if !path.starts_with(prefix) && !path.starts_with(&format!("{}/", prefix)) {
            return false;
        }
        let after_prefix = path.strip_prefix(prefix).unwrap_or(path.as_str());
        let after_prefix = after_prefix.strip_prefix('/').unwrap_or(after_prefix);
        return glob_match(after_prefix, suffix);
    }
    if let Some(ext) = pattern.strip_prefix("*.") {
        return path.ends_with(&format!(".{}", ext));
    }
    if !pattern.contains('*') {
        return path == pattern;
    }

    let parts: Vec<&str> = pattern
        .split('*')
        .filter(|segment| !segment.is_empty())
        .collect();
    if parts.is_empty() {
        return true;
    }

    let mut remaining = path.as_str();
    for part in parts {
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
    }
    true
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{TraceabilityMatcher, allows_unsupported_matches, glob_match};

    #[test]
    fn explicit_extension_glob_allows_unsupported_files() {
        let (matcher, warnings) = TraceabilityMatcher::new(
            Path::new("/repo"),
            &[String::from(".github/workflows/**/*.yml")],
            &[],
        );
        assert!(warnings.is_empty());
        assert!(matcher.matches(Path::new("/repo/.github/workflows/release.yml")));
    }

    #[test]
    fn broad_glob_does_not_allow_unsupported_files() {
        let (matcher, warnings) =
            TraceabilityMatcher::new(Path::new("/repo"), &[String::from("src/**")], &[]);
        assert!(warnings.is_empty());
        assert!(!matcher.matches(Path::new("/repo/src/config.toml")));
    }

    #[test]
    fn exact_file_allows_unsupported_files() {
        let (matcher, warnings) =
            TraceabilityMatcher::new(Path::new("/repo"), &[String::from("Cargo.toml")], &[]);
        assert!(warnings.is_empty());
        assert!(matcher.matches(Path::new("/repo/Cargo.toml")));
    }

    #[test]
    fn glob_match_supports_nested_extension_patterns() {
        assert!(glob_match(
            ".github/workflows/release.yml",
            ".github/workflows/**/*.yml"
        ));
        assert!(!glob_match(
            ".github/actions/release.yml",
            ".github/workflows/**/*.yml"
        ));
    }

    #[test]
    fn extension_specific_patterns_are_marked_explicit() {
        assert!(allows_unsupported_matches("**/*.yml"));
        assert!(allows_unsupported_matches("Cargo.toml"));
        assert!(!allows_unsupported_matches("src/**"));
    }
}
