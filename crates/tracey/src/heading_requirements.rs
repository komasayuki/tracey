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
}

#[derive(Debug, Clone)]
struct ExtractedHeadingReq {
    req: ReqDefinition,
}

/// 見出し `### r[req.id]` 形式を追加で抽出する `marq::render` ラッパー。
// r[impl req.markdown.section.support]
pub async fn render_with_heading_requirements(
    markdown: &str,
    options: &RenderOptions,
) -> marq::Result<Document> {
    let mut doc = marq::render(markdown, options).await?;
    let mut extracted = extract_heading_requirements(markdown).await?;
    if extracted.is_empty() {
        return Ok(doc);
    }

    let mut seen_ids: HashSet<RuleId> = doc.reqs.iter().map(|r| r.id.clone()).collect();
    let mut seen_bases: HashSet<String> = doc.reqs.iter().map(|r| r.id.base.clone()).collect();

    for entry in extracted.drain(..) {
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
        doc.reqs.push(entry.req.clone());
        doc.elements.push(DocElement::Req(entry.req));
    }

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
    Ok(ExtractedHeadingReq { req })
}

fn parse_heading(line: &str) -> Option<(usize, usize, &str)> {
    let line = line.trim_end_matches(['\n', '\r']);
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut i = 0usize;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    let hash_start = i;
    while i < bytes.len() && bytes[i] == b'#' {
        i += 1;
    }
    if i == hash_start || i >= bytes.len() || !matches!(bytes[i], b' ' | b'\t') {
        return None;
    }
    let level = i - hash_start;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    Some((level, i, &line[i..]))
}

fn update_fence_state(line: &str, in_fence: &mut Option<(char, usize)>) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    let Some((ch, count)) = parse_fence(trimmed) else {
        return false;
    };

    match in_fence {
        Some((open_ch, open_count)) if *open_ch == ch && count >= *open_count => {
            *in_fence = None;
            true
        }
        Some(_) => false,
        None => {
            *in_fence = Some((ch, count));
            true
        }
    }
}

fn parse_fence(line: &str) -> Option<(char, usize)> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = 1 + chars.take_while(|c| *c == first).count();
    (count >= 3).then_some((first, count))
}

fn parse_req_leading_marker(text: &str) -> Option<(&str, &str, usize)> {
    let mut prefix_len = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            prefix_len += ch.len_utf8();
        } else {
            break;
        }
    }
    if prefix_len == 0 || text.as_bytes().get(prefix_len) != Some(&b'[') {
        return None;
    }
    let marker_end = text.find(']')?;
    if marker_end <= prefix_len + 1 {
        return None;
    }
    let prefix = &text[..prefix_len];
    let marker_content = &text[prefix_len + 1..marker_end];
    Some((prefix, marker_content, marker_end))
}

fn parse_req_marker(inner: &str) -> marq::Result<(RuleId, ReqMetadata)> {
    let inner = inner.trim();
    let (req_id, attrs_str) = match inner.find(' ') {
        Some(idx) => (&inner[..idx], inner[idx + 1..].trim()),
        None => (inner, ""),
    };
    let req_id = marq::parse_rule_id(req_id).ok_or_else(|| {
        Error::DuplicateReq("empty or invalid requirement identifier".to_string())
    })?;
    let mut metadata = ReqMetadata::default();

    for attr in attrs_str.split_whitespace() {
        let (key, value) = attr
            .split_once('=')
            .ok_or_else(|| Error::CodeBlockHandler {
                language: "req".to_string(),
                message: format!(
                    "invalid attribute format '{}' for requirement '{}', expected: key=value",
                    attr, req_id
                ),
            })?;
        match key {
            "status" => {
                metadata.status = Some(marq::ReqStatus::parse(value).ok_or_else(|| {
                    Error::CodeBlockHandler {
                        language: "req".to_string(),
                        message: format!(
                            "invalid status '{}' for requirement '{}', expected: draft, stable, deprecated, removed",
                            value, req_id
                        ),
                    }
                })?);
            }
            "level" => {
                metadata.level = Some(marq::ReqLevel::parse(value).ok_or_else(|| {
                    Error::CodeBlockHandler {
                        language: "req".to_string(),
                        message: format!(
                            "invalid level '{}' for requirement '{}', expected: must, should, may",
                            value, req_id
                        ),
                    }
                })?);
            }
            "since" => metadata.since = Some(value.to_string()),
            "until" => metadata.until = Some(value.to_string()),
            "tags" => {
                metadata.tags = value.split(',').map(|s| s.trim().to_string()).collect();
            }
            _ => {
                return Err(Error::CodeBlockHandler {
                    language: "req".to_string(),
                    message: format!(
                        "unknown attribute '{}' for requirement '{}', expected: status, level, since, until, tags",
                        key, req_id
                    ),
                });
            }
        }
    }

    Ok((req_id, metadata))
}
