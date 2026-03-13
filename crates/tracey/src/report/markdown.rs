use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use tracey_api::{ApiCodeRef, ApiRule};

use crate::data::DashboardData;
use crate::report::meta::ReportMeta;
use crate::rule_coverage_policy::{rule_needs_impl, rule_needs_verify};
use crate::server::CoverageStats;

pub(super) fn render(
    project_root: &Path,
    report_path: &Path,
    data: &DashboardData,
    report_meta: &ReportMeta,
) -> String {
    let report_dir = report_path.parent().unwrap_or(report_path);
    let all_rules: Vec<ApiRule> = data
        .forward_by_impl
        .values()
        .flat_map(|forward| forward.rules.iter().cloned())
        .collect();
    let stats = CoverageStats::from_rules(&all_rules);

    let mut out = String::new();
    let _ = writeln!(out, "# Coverage Report");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Generated At: {}", report_meta.generated_at);
    let _ = writeln!(out, "- Git Hash: {}", report_meta.git_hash);
    let _ = writeln!(out, "- Total Rules: {}", stats.total_rules);
    let _ = writeln!(
        out,
        "- IMPL Coverage: {}/{} ({:.1}%)",
        stats.impl_covered, stats.impl_total_rules, stats.impl_percent
    );
    let _ = writeln!(
        out,
        "- TEST Coverage: {}/{} ({:.1}%)",
        stats.verify_covered, stats.verify_total_rules, stats.verify_percent
    );
    let _ = writeln!(out);

    for ((spec, impl_name), forward) in &data.forward_by_impl {
        let _ = writeln!(
            out,
            "## Spec: {}, Impl: {}",
            escape_heading(spec),
            escape_heading(impl_name)
        );
        let _ = writeln!(out);
        let _ = writeln!(out, "| Requirement ID | IMPL | TEST |");
        let _ = writeln!(out, "| --- | --- | --- |");

        for rule in &forward.rules {
            let impl_refs = render_refs(
                project_root,
                report_dir,
                &rule.impl_refs,
                rule_needs_impl(&rule.id.base),
            );
            let verify_refs = render_refs(
                project_root,
                report_dir,
                &rule.verify_refs,
                rule_needs_verify(&rule.id.base),
            );
            let _ = writeln!(out, "| `{}` | {} | {} |", rule.id, impl_refs, verify_refs);
        }

        let _ = writeln!(out);
    }

    out
}

fn render_refs(
    project_root: &Path,
    report_dir: &Path,
    refs: &[ApiCodeRef],
    needed: bool,
) -> String {
    if !needed {
        return "Not Needed".to_string();
    }
    if refs.is_empty() {
        return "-".to_string();
    }

    refs.iter()
        .map(|code_ref| render_ref(project_root, report_dir, code_ref))
        .collect::<Vec<_>>()
        .join("<br>")
}

fn render_ref(project_root: &Path, report_dir: &Path, code_ref: &ApiCodeRef) -> String {
    let full_path = full_path(project_root, &code_ref.file);
    let relative_link = compute_relative_path(report_dir, &full_path);
    let relative_link = relative_link.to_string_lossy().replace('\\', "/");
    let link_target = format!("{relative_link}#L{}", code_ref.line);
    let link_text = format!("{}:{}", code_ref.file, code_ref.line);
    format!("[{}]({})", escape_link_text(&link_text), link_target)
}

fn full_path(project_root: &Path, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn compute_relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let mut common_len = 0;
    for (a, b) in from_components.iter().zip(to_components.iter()) {
        if a == b {
            common_len += 1;
        } else {
            break;
        }
    }

    let mut result = PathBuf::new();
    for _ in common_len..from_components.len() {
        result.push("..");
    }
    for component in &to_components[common_len..] {
        result.push(component.as_os_str());
    }
    result
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn escape_heading(value: &str) -> String {
    escape_cell(value)
}

fn escape_link_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tracey_api::ApiCodeRef;

    use super::{render_ref, render_refs};

    #[test]
    fn relative_links_are_based_on_report_directory() {
        let reference = ApiCodeRef {
            file: "src/lib.rs".to_string(),
            line: 42,
        };
        let got = render_ref(
            Path::new("/repo"),
            Path::new("/repo/docs/traceability_report_txt"),
            &reference,
        );
        assert_eq!(got, "[src/lib.rs:42](../../src/lib.rs#L42)");
    }

    #[test]
    fn not_needed_is_rendered_for_optional_side() {
        let got = render_refs(Path::new("/repo"), Path::new("/repo/out"), &[], false);
        assert_eq!(got, "Not Needed");
    }
}
