use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use tracey::data::{build_dashboard_data, render_spec_content_for_impl};
use tracey::server::QueryEngine;

fn setup_fixture() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    fs::create_dir_all(root.join(".config/tracey")).unwrap();
    fs::create_dir_all(root.join("docs/spec")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();

    // prefix ごとの必要 coverage を混在させる。
    let spec = r#"# Coverage Policy

r[impl.only.rule]
Impl only.

r[verify.only.rule]
Verify only.

r[shared.rule]
Needs both.

r[needs.verify.rule]
Needs verify.

r[missing.rule]
Needs both and is missing.
"#;
    fs::write(root.join("docs/spec/spec.md"), spec).unwrap();

    let source = r#"// r[impl impl.only.rule]
fn impl_only() {}

// r[verify verify.only.rule]
fn verify_only() {}

// r[impl shared.rule]
// r[verify shared.rule]
fn shared() {}

// r[impl needs.verify.rule]
fn needs_verify() {}
"#;
    fs::write(root.join("src/lib.rs"), source).unwrap();

    let config = r#"specs (
  {
    name test-spec
    include (docs/spec/**/*.md)
    impls (
      {
        name main
        include (src/**/*.rs)
      }
    )
  }
)
"#;
    fs::write(root.join(".config/tracey/config.styx"), config).unwrap();

    (tmp, root)
}

fn setup_nested_outline_fixture() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    fs::create_dir_all(root.join(".config/tracey")).unwrap();
    fs::create_dir_all(root.join("docs/spec")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();

    let spec = r#"# Parent Section

## Child Requirement Section

r[req.child.rule]
Child rule.
"#;
    fs::write(root.join("docs/spec/spec.md"), spec).unwrap();

    let source = r#"// r[impl req.child.rule]
// r[verify req.child.rule]
fn covered_child() {}
"#;
    fs::write(root.join("src/lib.rs"), source).unwrap();

    let config = r#"specs (
  {
    name test-spec
    include (docs/spec/**/*.md)
    impls (
      {
        name main
        include (src/**/*.rs)
      }
    )
  }
)
"#;
    fs::write(root.join(".config/tracey/config.styx"), config).unwrap();

    (tmp, root)
}

#[tokio::test]
async fn prefix_requirements_use_prefix_aware_coverage_totals() {
    let (_tmp, root) = setup_fixture();
    let config_path = root.join(".config/tracey/config.styx");
    let config = tracey::load_config(&config_path).unwrap();
    let data = build_dashboard_data(&root, &config, 1, true).await.unwrap();
    let engine = QueryEngine::new(&data);

    let (_, _, stats) = &engine.status()[0];
    assert_eq!(stats.total_rules, 5);
    assert_eq!(stats.impl_total_rules, 4);
    assert_eq!(stats.verify_total_rules, 4);
    assert_eq!(stats.impl_covered, 3);
    assert_eq!(stats.verify_covered, 2);
    assert_eq!(stats.fully_covered, 3);

    let uncovered = engine.uncovered("test-spec", "main", None).unwrap();
    let uncovered_ids: Vec<_> = uncovered
        .by_section
        .values()
        .flat_map(|rules| rules.iter().map(|rule| rule.id.to_string()))
        .collect();
    assert_eq!(uncovered.total_uncovered, 1);
    assert!(uncovered_ids.contains(&"missing.rule".to_string()));
    assert!(!uncovered_ids.contains(&"verify.only.rule".to_string()));

    let untested = engine.untested("test-spec", "main", None).unwrap();
    let untested_ids: Vec<_> = untested
        .by_section
        .values()
        .flat_map(|rules| rules.iter().map(|rule| rule.id.to_string()))
        .collect();
    assert_eq!(untested.total_untested, 2);
    assert!(untested_ids.contains(&"needs.verify.rule".to_string()));
    assert!(untested_ids.contains(&"missing.rule".to_string()));
    assert!(!untested_ids.contains(&"impl.only.rule".to_string()));
}

#[tokio::test]
async fn prefix_requirements_flow_into_spec_outline_totals() {
    let (_tmp, root) = setup_fixture();
    let config_path = root.join(".config/tracey/config.styx");
    let config = tracey::load_config(&config_path).unwrap();
    let data = build_dashboard_data(&root, &config, 1, true).await.unwrap();
    let forward = data
        .forward_by_impl
        .get(&("test-spec".to_string(), "main".to_string()))
        .unwrap();
    let include = vec!["docs/spec/**/*.md".to_string()];
    let spec = render_spec_content_for_impl(&root, &include, "test-spec", "main", forward)
        .await
        .unwrap();

    let root_entry = spec.outline.first().unwrap();
    assert_eq!(root_entry.aggregated.total, 5);
    assert_eq!(root_entry.aggregated.impl_total, 4);
    assert_eq!(root_entry.aggregated.verify_total, 4);
    assert_eq!(root_entry.aggregated.impl_count, 3);
    assert_eq!(root_entry.aggregated.verify_count, 2);
}

#[tokio::test]
async fn outline_parent_aggregates_child_totals_even_without_direct_rules() {
    let (_tmp, root) = setup_nested_outline_fixture();
    let config_path = root.join(".config/tracey/config.styx");
    let config = tracey::load_config(&config_path).unwrap();
    let data = build_dashboard_data(&root, &config, 1, true).await.unwrap();
    let forward = data
        .forward_by_impl
        .get(&("test-spec".to_string(), "main".to_string()))
        .unwrap();
    let include = vec!["docs/spec/**/*.md".to_string()];
    let spec = render_spec_content_for_impl(&root, &include, "test-spec", "main", forward)
        .await
        .unwrap();

    let parent = &spec.outline[0];
    let child = &spec.outline[1];

    assert_eq!(parent.coverage.total, 0);
    assert_eq!(parent.aggregated.total, 1);
    assert_eq!(parent.aggregated.impl_total, 1);
    assert_eq!(parent.aggregated.verify_total, 1);
    assert_eq!(parent.aggregated.impl_count, 1);
    assert_eq!(parent.aggregated.verify_count, 1);

    assert_eq!(child.coverage.total, 1);
    assert_eq!(child.aggregated.total, 1);
}
