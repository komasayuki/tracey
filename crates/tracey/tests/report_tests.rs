use std::fs;
use std::path::Path;

use tempfile::tempdir;

#[tokio::test]
async fn report_command_generates_markdown_file() {
    let temp = tempdir().expect("temporary directory");
    let root = temp.path();

    fs::create_dir_all(root.join(".config/tracey")).unwrap();
    fs::create_dir_all(root.join("docs/spec")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(
        root.join(".config/tracey/config.styx"),
        r#"specs (
  {
    name main
    include (docs/spec/**/*.md)
    impls (
      {
        name rust
        include (src/**/*.rs)
      }
    )
  }
)
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/spec/requirements.md"),
        "# Requirements\n\nr[req.example.rule]\n\nr[verify.example.check]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "// r[impl req.example.rule]\n// r[verify req.example.rule]\nfn both() {}\n\n// r[verify verify.example.check]\nfn verify_only() {}\n",
    )
    .unwrap();

    tracey::report::run(
        Some(root.to_path_buf()),
        Some(Path::new("docs/traceability_report_txt").to_path_buf()),
    )
    .await
    .unwrap();

    let report_path = root.join("docs/traceability_report_txt/traceability_report.md");
    let report = fs::read_to_string(&report_path).unwrap();

    assert!(report.starts_with("# Coverage Report"));
    assert!(report.contains("- Git Hash: -"));
    assert!(report.contains("- Total Rules: 2"));
    assert!(report.contains("- IMPL Coverage: 1/1 (100.0%)"));
    assert!(report.contains("- TEST Coverage: 2/2 (100.0%)"));
    assert!(report.contains("## Spec: main, Impl: rust"));
    assert!(report.contains("| `req.example.rule` |"));
    assert!(report.contains("[src/lib.rs:"));
    assert!(report.contains("(../../src/lib.rs#L"));
    assert!(report.contains("| `verify.example.check` | Not Needed |"));
}
