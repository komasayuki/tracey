use marq::{Error, ReqMetadata, RuleId};

pub(crate) fn parse_heading(line: &str) -> Option<(usize, usize, &str)> {
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

pub(crate) fn update_fence_state(line: &str, in_fence: &mut Option<(char, usize)>) -> bool {
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

pub(crate) fn parse_req_leading_marker(text: &str) -> Option<(&str, &str, usize)> {
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

pub(crate) fn parse_req_marker(inner: &str) -> marq::Result<(RuleId, ReqMetadata)> {
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
