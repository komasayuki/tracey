use marq::{Error, RenderOptions};
use std::path::Path;

#[tokio::test]
async fn extracts_multiline_heading_requirement() {
    let md = r#"
### r[req.type.numbers]
数字を入力したら、数字が入力される

- 777 と入力した場合は、特別にLuckyと入力する
#### 補足
- 222 と入力した場合は、特別にCatと入力する

### r[req.type.others]
その他の入力に対しては、特別な処理を行わない。
"#;
    let doc = tracey::heading_requirements::render_with_heading_requirements(
        md,
        &RenderOptions::default(),
    )
    .await
    .unwrap();

    assert_eq!(doc.reqs.len(), 2);
    assert!(doc.reqs[0].raw.contains("777"));
    assert!(doc.reqs[0].raw.contains("#### 補足"));
    assert!(doc.reqs[1].raw.contains("その他の入力"));
}

#[tokio::test]
async fn closes_on_higher_or_same_heading() {
    let md = r#"
### r[req.alpha]
A
## 区切り
B
"#;
    let doc = tracey::heading_requirements::render_with_heading_requirements(
        md,
        &RenderOptions::default(),
    )
    .await
    .unwrap();

    assert_eq!(doc.reqs.len(), 1);
    assert_eq!(doc.reqs[0].raw.trim(), "A");
}

#[tokio::test]
async fn rejects_duplicate_ids_between_formats() {
    let md = r#"
r[req.same] 既存形式

### r[req.same]
見出し形式
"#;
    let err = tracey::heading_requirements::render_with_heading_requirements(
        md,
        &RenderOptions::default(),
    )
    .await
    .expect_err("重複IDで失敗するはず");
    assert!(matches!(err, Error::DuplicateReq(_)));
}

#[tokio::test]
// r[validate req.markdown.section.support]
async fn recognizes_requirement_in_patch_requirements_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = tracey::load_rules_from_glob(
        &root,
        "docs/content/spec/patch_requirements.md",
        true,
    )
    .await
    .expect("specの読み込みに失敗");

    let has_target = rules
        .iter()
        .any(|r| r.def.id.to_string() == "req.markdown.section.support");
    assert!(
        has_target,
        "req.markdown.section.support が抽出されるべき"
    );
}
