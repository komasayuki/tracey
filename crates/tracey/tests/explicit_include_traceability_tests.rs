use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tracey::data::build_dashboard_data;
use tracey_api::ValidationErrorCode;
use tracey_core::parse_rule_id;

fn write_project(config: &str, spec: &str, cargo_toml: &str, workflow: &str) -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();

    fs::create_dir_all(root.join(".config/tracey")).unwrap();
    fs::create_dir_all(root.join("docs/spec")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();

    fs::write(root.join(".config/tracey/config.styx"), config).unwrap();
    fs::write(root.join("docs/spec/requirements.md"), spec).unwrap();
    fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(root.join(".github/workflows/check.yml"), workflow).unwrap();
    fs::write(root.join("src/lib.rs"), "fn placeholder() {}\n").unwrap();

    (temp, root)
}

async fn load_dashboard(root: &Path) -> tracey::data::DashboardData {
    let config_path = root.join(".config/tracey/config.styx");
    let config = tracey::load_config(&config_path).unwrap();
    build_dashboard_data(root, &config, 1, true).await.unwrap()
}

fn forward_rule<'a>(
    data: &'a tracey::data::DashboardData,
    rule_id: &str,
) -> &'a tracey_api::ApiRule {
    data.forward_by_impl
        .get(&(String::from("spec"), String::from("main")))
        .unwrap()
        .rules
        .iter()
        .find(|rule| rule.id == parse_rule_id(rule_id).expect("valid rule id"))
        .unwrap()
}

#[tokio::test]
async fn explicit_include_scans_toml_for_impl_and_verify() {
    let config = r#"specs (
  {
    name spec
    include (docs/spec/**/*.md)
    impls (
      {
        name main
        include (src/**/*.rs Cargo.toml)
      }
    )
  }
)
"#;
    let spec = "# Requirements\n\nr[req.config.file]\n";
    let cargo_toml =
        "# r[impl req.config.file]\n# r[verify req.config.file]\n[package]\nname='fixture'\n";
    let (_temp, root) = write_project(config, spec, cargo_toml, "name: check\n");

    let data = load_dashboard(&root).await;
    let rule = forward_rule(&data, "req.config.file");

    assert_eq!(rule.impl_refs.len(), 1);
    assert_eq!(rule.verify_refs.len(), 1);
    assert_eq!(rule.impl_refs[0].file, "Cargo.toml");
    assert_eq!(rule.verify_refs[0].file, "Cargo.toml");
}

#[tokio::test]
async fn test_include_scans_yaml_for_verify_and_flags_impl() {
    let config = r#"specs (
  {
    name spec
    include (docs/spec/**/*.md)
    impls (
      {
        name main
        include (src/**/*.rs)
        test_include (.github/workflows/**/*.yml)
      }
    )
  }
)
"#;
    let spec = "# Requirements\n\nr[req.workflow.check]\n";
    let workflow = "# r[verify req.workflow.check]\n# r[impl req.workflow.check]\nname: check\n";
    let (_temp, root) = write_project(config, spec, "[package]\nname='fixture'\n", workflow);

    let data = load_dashboard(&root).await;
    let rule = forward_rule(&data, "req.workflow.check");
    let validation = data
        .validation_by_impl
        .get(&(String::from("spec"), String::from("main")))
        .unwrap();

    assert_eq!(rule.verify_refs.len(), 1);
    assert_eq!(rule.verify_refs[0].file, ".github/workflows/check.yml");
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.code == ValidationErrorCode::ImplInTestFile)
    );
}

#[tokio::test]
async fn unsupported_extension_is_not_scanned_without_explicit_include() {
    let config = r#"specs (
  {
    name spec
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
    let spec = "# Requirements\n\nr[req.config.file]\n";
    let cargo_toml = "# r[impl req.config.file]\n[package]\nname='fixture'\n";
    let (_temp, root) = write_project(config, spec, cargo_toml, "name: check\n");

    let data = load_dashboard(&root).await;
    let rule = forward_rule(&data, "req.config.file");

    assert!(rule.impl_refs.is_empty());
    assert!(rule.verify_refs.is_empty());
}
