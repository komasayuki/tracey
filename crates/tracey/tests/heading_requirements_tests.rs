use marq::{Error, RenderOptions};
use std::path::Path;
use std::{future::Future, pin::Pin};
use tracey::data::render_spec_content_for_impl;
use tracey_api::ApiSpecForward;

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

// r[validate req.markdown.section.support]
#[tokio::test]
async fn recognizes_requirement_in_patch_requirements_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules =
        tracey::load_rules_from_glob(&root, "docs/content/spec/patch_requirements.md", true)
            .await
            .expect("specの読み込みに失敗");

    let has_target = rules
        .iter()
        .any(|r| r.def.id.base == "req.markdown.section.support");
    assert!(has_target, "req.markdown.section.support が抽出されるべき");
}

struct MarkerReqHandler;

impl marq::ReqHandler for MarkerReqHandler {
    fn start<'a>(
        &'a self,
        req: &'a marq::ReqDefinition,
    ) -> Pin<Box<dyn Future<Output = marq::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            Ok(format!(
                r#"<div class="req-container" data-rule="{}"><div class="req-content">"#,
                req.id
            ))
        })
    }

    fn end<'a>(
        &'a self,
        _req: &'a marq::ReqDefinition,
    ) -> Pin<Box<dyn Future<Output = marq::Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok("</div></div>".to_string()) })
    }
}

#[tokio::test]
async fn wraps_heading_requirement_in_html_view() {
    let md = r#"
# Spec

## r[req.markdown.section.support]
このセクションは要件として扱う。

通常の本文。

## Next
次のセクション
"#;
    let opts = RenderOptions::default().with_req_handler(MarkerReqHandler);
    let doc = tracey::heading_requirements::render_with_heading_requirements(md, &opts)
        .await
        .unwrap();

    assert!(
        doc.html
            .contains(r#"data-rule="req.markdown.section.support""#)
    );
    let req_pos = doc
        .html
        .find(r#"data-rule="req.markdown.section.support""#)
        .unwrap();
    let heading_pos = doc.html.find("<h2").unwrap();
    assert!(
        req_pos < heading_pos,
        "要件コンテナは見出しの手前に入るべき"
    );
}

#[tokio::test]
async fn wraps_heading_requirement_in_web_spec_content() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let include = vec!["docs/content/spec/patch_requirements.md".to_string()];
    let forward = ApiSpecForward {
        name: "patch".to_string(),
        rules: Vec::new(),
    };
    let spec = render_spec_content_for_impl(&root, &include, "patch", "rust", &forward)
        .await
        .expect("spec content render failed");

    let html = &spec.sections[0].html;
    let heading = r#"<h2 id="patch-requirements--r-req.markdown.section.support">"#;
    let heading_pos = html.find(heading).expect("target heading not found");
    let before_heading = &html[..heading_pos];
    let last_open = before_heading
        .rfind(r#"class="req-container"#)
        .expect("req-container open not found before heading");
    let last_close = before_heading.rfind("</div>\n</div>").unwrap_or(0);
    assert!(
        last_open > last_close,
        "target heading must be inside an open req-container"
    );
    assert!(html.contains("req.markdown.section.support"));
}

#[tokio::test]
async fn wraps_even_when_heading_ids_repeat() {
    let md = r#"
# Repeat
## End
before

# Repeat
## r[req.markdown.section.support]
target body

## End
after
"#;
    let opts = RenderOptions::default().with_req_handler(MarkerReqHandler);
    let doc = tracey::heading_requirements::render_with_heading_requirements(md, &opts)
        .await
        .unwrap();

    assert!(
        doc.html
            .contains(r#"data-rule="req.markdown.section.support""#),
        "duplicate heading id があっても要件がラップされるべき"
    );
}

#[tokio::test]
async fn ignores_requirement_markers_inside_markdown_links() {
    let md = r#"
r[req.abc.cde]
See [r[req.abc.cde]](#r[req.abc.cde]) for navigation.
"#;
    let doc = tracey::heading_requirements::render_with_heading_requirements(
        md,
        &RenderOptions::default(),
    )
    .await
    .unwrap();

    assert_eq!(doc.reqs.len(), 1, "リンク内の r[...] は定義扱いしない");
    assert_eq!(doc.reqs[0].id.to_string(), "req.abc.cde");
    assert!(doc.reqs[0].raw.contains("r[req.abc.cde]"));
    assert!(
        doc.html
            .contains(r##"<a href="#r[req.abc.cde]">r[req.abc.cde]</a>"##)
    );
}

#[tokio::test]
async fn preserves_requirement_link_text_inside_heading_requirement_body() {
    let md = r#"
#### r[req.gateway.set]
[r[req.gateway.start]](#rreqgatewaystart) の後に呼び出せる。

#### r[req.gateway.start]
Gateway を開始する。
"#;
    let doc = tracey::heading_requirements::render_with_heading_requirements(
        md,
        &RenderOptions::default(),
    )
    .await
    .unwrap();

    assert!(
        doc.reqs[0]
            .html
            .contains(r##"<a href="#rreqgatewaystart">r[req.gateway.start]</a>"##)
    );
    assert!(
        !doc.reqs[0]
            .html
            .contains(r##"<a href="#rreqgatewaystart"></a>"##)
    );
}

#[tokio::test]
async fn generates_github_style_requirement_anchor_alias_in_spec_view() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let spec_path = temp.path().join("spec.md");
    std::fs::write(
        &spec_path,
        r#"
# Spec

#### r[req.gateway.set]
[r[req.gateway.start]](#rreqgatewaystart) の後に呼び出せる。

#### r[req.gateway.start]
Gateway を開始する。
"#,
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
    .expect("spec content render failed");

    let html = &spec.sections[0].html;
    assert!(html.contains(r##"href="#rreqgatewaystart">r[req.gateway.start]</a>"##));
    assert!(html.contains(r#"id="rreqgatewaystart""#));
}
