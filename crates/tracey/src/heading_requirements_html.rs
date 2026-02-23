use marq::{Document, Heading, RenderOptions, ReqDefinition};

/// 見出し要件セクションに req コンテナを挿入する。
pub(crate) async fn inject_heading_requirement_containers(
    doc: &mut Document,
    entries: &[(ReqDefinition, usize, usize)],
    options: &RenderOptions,
) -> marq::Result<()> {
    if entries.is_empty() || doc.headings.is_empty() || doc.html.is_empty() {
        return Ok(());
    }

    let heading_starts = locate_heading_starts_in_order(&doc.html, &doc.headings);

    let mut targets = Vec::new();
    for (req, heading_level, heading_index) in entries {
        let heading_idx = resolve_heading_index(&doc.headings, req, *heading_level, *heading_index);
        let Some(start_idx) = heading_starts.get(heading_idx).copied().flatten() else {
            continue;
        };

        let end_idx =
            find_section_end(&doc.headings, &heading_starts, heading_idx).unwrap_or(doc.html.len());
        if end_idx <= start_idx {
            continue;
        }

        let (wrap_start, wrap_end) = req_wrapper_html(req, options).await?;
        targets.push((start_idx, end_idx, wrap_start, wrap_end));
    }

    if targets.is_empty() {
        return Ok(());
    }

    // 重なりがあるとインデックス補正が複雑になるため、重なる対象は後勝ちで落とす。
    targets.sort_by_key(|(start, _, _, _)| *start);
    let mut non_overlapping = Vec::new();
    let mut last_end = 0usize;
    for target in targets {
        if target.0 < last_end {
            continue;
        }
        last_end = target.1;
        non_overlapping.push(target);
    }

    for (start_idx, end_idx, wrap_start, wrap_end) in non_overlapping.into_iter().rev() {
        doc.html.insert_str(end_idx, &wrap_end);
        doc.html.insert_str(start_idx, &wrap_start);
    }

    Ok(())
}

fn resolve_heading_index(
    headings: &[Heading],
    req: &ReqDefinition,
    heading_level: usize,
    heading_index: usize,
) -> usize {
    if heading_index_is_valid(headings, heading_level, req, heading_index) {
        return heading_index;
    }
    find_heading_for_req(headings, req, heading_level).unwrap_or(heading_index)
}

fn heading_index_is_valid(
    headings: &[Heading],
    heading_level: usize,
    req: &ReqDefinition,
    heading_index: usize,
) -> bool {
    headings
        .get(heading_index)
        .is_some_and(|h| h.level as usize == heading_level && h.line == req.line)
}

fn find_heading_for_req(
    headings: &[Heading],
    req: &ReqDefinition,
    heading_level: usize,
) -> Option<usize> {
    headings
        .iter()
        .enumerate()
        .find(|(_, h)| h.line == req.line && h.level as usize == heading_level)
        .map(|(idx, _)| idx)
}

fn locate_heading_starts_in_order(html: &str, headings: &[Heading]) -> Vec<Option<usize>> {
    let mut cursor = 0usize;
    let mut starts = Vec::with_capacity(headings.len());
    for heading in headings {
        let token = format!(r#"<h{} id="{}">"#, heading.level, heading.id);
        if let Some(rel) = html[cursor..].find(&token) {
            let abs = cursor + rel;
            starts.push(Some(abs));
            cursor = abs + token.len();
        } else {
            starts.push(None);
        }
    }
    starts
}

fn find_section_end(
    headings: &[Heading],
    heading_starts: &[Option<usize>],
    heading_idx: usize,
) -> Option<usize> {
    let current_level = headings.get(heading_idx)?.level;
    let next_idx = headings
        .iter()
        .skip(heading_idx + 1)
        .position(|h| h.level <= current_level)
        .map(|i| i + heading_idx + 1)?;
    heading_starts.get(next_idx).copied().flatten()
}

async fn req_wrapper_html(
    req: &ReqDefinition,
    options: &RenderOptions,
) -> marq::Result<(String, String)> {
    let default_req_handler: marq::BoxedReqHandler = std::sync::Arc::new(marq::DefaultReqHandler);
    let req_handler = options.req_handler.as_ref().unwrap_or(&default_req_handler);
    let start = req_handler.start(req).await?;
    let end = req_handler.end(req).await?;
    Ok((start, end))
}
