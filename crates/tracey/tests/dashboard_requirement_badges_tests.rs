use std::fs;

use tempfile::tempdir;
use tracey::data::render_spec_content_for_impl;
use tracey_api::ApiSpecForward;

#[tokio::test]
async fn verify_prefixed_rules_show_impl_not_needed_badge() {
    let temp = tempdir().expect("temporary directory");
    fs::write(
        temp.path().join("spec.md"),
        "# Spec\n\n#### r[verify.gateway.test]\nOnly verify is needed.\n",
    )
    .expect("write spec");

    let spec = render_spec_content_for_impl(
        temp.path(),
        &["spec.md".to_string()],
        "demo",
        "rust",
        &ApiSpecForward {
            name: "demo".to_string(),
            rules: Vec::new(),
        },
    )
    .await
    .expect("render spec content");

    let html = &spec.sections[0].html;
    assert!(html.contains(r#"class="req-badge req-impl req-not-needed""#));
    assert!(html.contains(r#">Not Needed</span>"#));
}

#[tokio::test]
async fn impl_prefixed_rules_show_verify_not_needed_badge() {
    let temp = tempdir().expect("temporary directory");
    fs::write(
        temp.path().join("spec.md"),
        "# Spec\n\n#### r[impl.gateway.runtime]\nOnly impl is needed.\n",
    )
    .expect("write spec");

    let spec = render_spec_content_for_impl(
        temp.path(),
        &["spec.md".to_string()],
        "demo",
        "rust",
        &ApiSpecForward {
            name: "demo".to_string(),
            rules: Vec::new(),
        },
    )
    .await
    .expect("render spec content");

    let html = &spec.sections[0].html;
    assert!(html.contains(r#"class="req-badge req-test req-not-needed""#));
    assert!(html.contains(r#">Not Needed</span>"#));
}
