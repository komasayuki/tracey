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

    assert!(report.starts_with("# Requirements traceability report"));
    assert!(report.contains("Generated At: "));
    assert!(report.contains("Git Hash: -"));
    assert!(report.contains("## Coverage"));
    assert!(report.contains("Total Rules: 2"));
    assert!(report.contains("IMPL Coverage: 1/1 (100.0%)"));
    assert!(report.contains("TEST Coverage: 2/2 (100.0%)"));
    assert!(report.contains("## Spec: main, Impl: rust"));
    assert!(report.contains("| Requirement ID | Implemented At | Tested At |"));
    assert!(report.contains("| `req.example.rule` |"));
    assert!(report.contains("[src/lib.rs:"));
    assert!(report.contains("(../../src/lib.rs#L"));
    assert!(report.contains("| `verify.example.check` | Not Needed |"));
}

#[tokio::test]
async fn report_sorts_tables_and_requirement_ids() {
    let temp = tempdir().expect("temporary directory");
    let root = temp.path();

    fs::create_dir_all(root.join(".config/tracey")).unwrap();
    fs::create_dir_all(root.join("docs/spec-alpha")).unwrap();
    fs::create_dir_all(root.join("docs/spec-zeta")).unwrap();
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
    name zeta
    include (docs/spec-zeta/**/*.md)
    impls (
      {
        name b
        include (src/**/*.rs)
      }
    )
  }
  {
    name alpha
    include (docs/spec-alpha/**/*.md)
    impls (
      {
        name a
        include (src/**/*.rs)
      }
    )
  }
)
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/spec-alpha/requirements.md"),
        "# Alpha\n\na[req.alpha.z]\n\na[req.alpha.a]\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/spec-zeta/requirements.md"),
        "# Zeta\n\nz[req.zeta.b]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "// a[impl req.alpha.z]\nfn alpha_z() {}\n\n// a[impl req.alpha.a]\nfn alpha_a() {}\n\n// z[impl req.zeta.b]\nfn zeta_b() {}\n",
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

    let alpha_table = report.find("## Spec: alpha, Impl: a").unwrap();
    let zeta_table = report.find("## Spec: zeta, Impl: b").unwrap();
    assert!(alpha_table < zeta_table);

    let alpha_a = report.find("| `req.alpha.a` |").unwrap();
    let alpha_z = report.find("| `req.alpha.z` |").unwrap();
    assert!(alpha_a < alpha_z);
}
