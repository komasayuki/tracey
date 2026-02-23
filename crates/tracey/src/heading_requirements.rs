use crate::heading_requirements_parse::{
    parse_heading, parse_req_leading_marker, parse_req_marker, update_fence_state,
};
use marq::{DocElement, Document, Error, RenderOptions, ReqDefinition, ReqMetadata, RuleId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct HeadingReqStart {
    level: usize,
    line: usize,
    marker_offset: usize,
    marker_length: usize,
    prefix: String,
    req_id: RuleId,
    metadata: ReqMetadata,
    body_start_offset: usize,
    heading_index: usize,
}

#[derive(Debug, Clone)]
struct ExtractedHeadingReq {
    req: ReqDefinition,
    heading_level: usize,
    heading_index: usize,
}

/// 見出し `### r[req.id]` 形式を追加で抽出する `marq::render` ラッパー。
// r[impl req.markdown.section.support]
pub async fn render_with_heading_requirements(
    markdown: &str,
    options: &RenderOptions,
) -> marq::Result<Document> {
    let mut doc = marq::render(markdown, options).await?;
    let extracted = extract_heading_requirements(markdown).await?;
    if extracted.is_empty() {
        return Ok(doc);
    }

    let mut seen_ids: HashSet<RuleId> = doc.reqs.iter().map(|r| r.id.clone()).collect();
    let mut seen_bases: HashSet<String> = doc.reqs.iter().map(|r| r.id.base.clone()).collect();

    for entry in &extracted {
        if seen_ids.contains(&entry.req.id) {
            return Err(Error::DuplicateReq(entry.req.id.to_string()));
        }
        if seen_bases.contains(&entry.req.id.base) {
            return Err(Error::DuplicateReq(format!(
                "duplicate requirement base: {}",
                entry.req.id.base
            )));
        }
        seen_ids.insert(entry.req.id.clone());
        seen_bases.insert(entry.req.id.base.clone());
    }

    for entry in &extracted {
        doc.reqs.push(entry.req.clone());
        doc.elements.push(DocElement::Req(entry.req.clone()));
    }

    let wrap_entries: Vec<(ReqDefinition, usize, usize)> = extracted
        .iter()
        .map(|e| (e.req.clone(), e.heading_level, e.heading_index))
        .collect();
    crate::heading_requirements_html::inject_heading_requirement_containers(
        &mut doc,
        &wrap_entries,
        options,
    )
    .await?;

    sort_elements_by_line(&mut doc.elements);
    Ok(doc)
}

fn sort_elements_by_line(elements: &mut Vec<DocElement>) {
    let mut indexed: Vec<(usize, DocElement)> = elements.drain(..).enumerate().collect();
    indexed.sort_by_key(|(idx, elem)| {
        let (line, rank) = match elem {
            DocElement::Heading(h) => (h.line, 0usize),
            DocElement::Req(r) => (r.line, 1usize),
            DocElement::Paragraph(p) => (p.line, 2usize),
        };
        (line, rank, *idx)
    });
    *elements = indexed.into_iter().map(|(_, elem)| elem).collect();
}

async fn extract_heading_requirements(markdown: &str) -> marq::Result<Vec<ExtractedHeadingReq>> {
    let mut results = Vec::new();
    let mut current: Option<HeadingReqStart> = None;
    let mut in_fence: Option<(char, usize)> = None;
    let mut line_no = 1usize;
    let mut offset = 0usize;
    let mut heading_index = 0usize;

    for line in markdown.split_inclusive('\n') {
        // コードフェンス内の `#` は見出しとして扱わない。
        if update_fence_state(line, &mut in_fence) {
            offset += line.len();
            line_no += 1;
            continue;
        }
        if in_fence.is_some() {
            offset += line.len();
            line_no += 1;
            continue;
        }

        let (level, content_start, content) = match parse_heading(line) {
            Some(v) => v,
            None => {
                offset += line.len();
                line_no += 1;
                continue;
            }
        };
        let current_heading_index = heading_index;
        heading_index += 1;

        // 同一レベル or 上位レベルの見出しが出たら、現在の要件を閉じる。
        if let Some(open) = current.take() {
            if level <= open.level {
                results.push(build_heading_req(markdown, open, offset).await?);
            } else {
                current = Some(open);
            }
        }

        let trimmed_content = content.trim_start();
        let skipped = content.len().saturating_sub(trimmed_content.len());
        if let Some((prefix, marker_content, marker_end)) =
            parse_req_leading_marker(trimmed_content)
        {
            let (req_id, metadata) = parse_req_marker(marker_content)?;
            let marker_offset = offset + content_start + skipped;
            current = Some(HeadingReqStart {
                level,
                line: line_no,
                marker_offset,
                marker_length: marker_end + 1,
                prefix: prefix.to_string(),
                req_id,
                metadata,
                body_start_offset: offset + line.len(),
                heading_index: current_heading_index,
            });
        }

        offset += line.len();
        line_no += 1;
    }

    if let Some(open) = current.take() {
        results.push(build_heading_req(markdown, open, markdown.len()).await?);
    }

    Ok(results)
}

async fn build_heading_req(
    markdown: &str,
    open: HeadingReqStart,
    end_offset: usize,
) -> marq::Result<ExtractedHeadingReq> {
    let anchor_id = format!("{}-{}", open.prefix, open.req_id);
    let raw = markdown[open.body_start_offset..end_offset]
        .trim_end()
        .to_string();
    let html = if raw.is_empty() {
        String::new()
    } else {
        marq::render(&raw, &RenderOptions::default()).await?.html
    };
    let req = ReqDefinition {
        id: open.req_id,
        anchor_id,
        marker_span: marq::SourceSpan {
            offset: open.marker_offset,
            length: open.marker_length,
        },
        span: marq::SourceSpan {
            offset: open.marker_offset,
            length: end_offset.saturating_sub(open.marker_offset),
        },
        line: open.line,
        metadata: open.metadata,
        raw,
        html,
    };
    Ok(ExtractedHeadingReq {
        req,
        heading_level: open.level,
        heading_index: open.heading_index,
    })
}
