use std::path::Path;

use tracey_core::{RefVerb, Reqs};

#[test]
fn unsupported_extension_uses_text_fallback_for_hash_comments() {
    let content = "# r[impl req.config.file]\nkey = \"value\"\n";
    let reqs = Reqs::extract_from_content(Path::new("Cargo.toml"), content);

    assert_eq!(reqs.references.len(), 1);
    assert_eq!(reqs.references[0].verb, RefVerb::Impl);
    assert_eq!(reqs.references[0].req_id.to_string(), "req.config.file");
}

#[test]
fn unsupported_extension_supports_verify_hash_comments() {
    let content = "# r[verify req.workflow.check]\nname: workflow\n";
    let reqs = Reqs::extract_from_content(Path::new("workflow.yml"), content);

    assert_eq!(reqs.references.len(), 1);
    assert_eq!(reqs.references[0].verb, RefVerb::Verify);
    assert_eq!(reqs.references[0].req_id.to_string(), "req.workflow.check");
}

#[test]
fn unsupported_extension_supports_underscore_in_rule_ids() {
    let content = "# r[impl req.logging.log_compatibility]\n[package]\nname='fixture'\n";
    let reqs = Reqs::extract_from_content(Path::new("Cargo.toml"), content);

    assert_eq!(reqs.references.len(), 1);
    assert_eq!(reqs.references[0].verb, RefVerb::Impl);
    assert_eq!(
        reqs.references[0].req_id.to_string(),
        "req.logging.log_compatibility"
    );
}
