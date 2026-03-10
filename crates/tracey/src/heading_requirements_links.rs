use crate::heading_requirements_parse::parse_req_marker;
use marq::{DocElement, Document};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;

#[derive(Debug, Default)]
pub(crate) struct SanitizedMarkdown {
    pub(crate) markdown: String,
    pub(crate) replacements: Vec<(String, String)>,
}

pub(crate) fn sanitize_link_markers(markdown: &str) -> SanitizedMarkdown {
    let mut bytes = markdown.as_bytes().to_vec();
    let mut replacements = Vec::new();

    for range in markdown_link_ranges(markdown) {
        let mut idx = range.start;
        while idx < range.end {
            if !bytes[idx].is_ascii_alphabetic() {
                idx += 1;
                continue;
            }
            if idx > range.start && bytes[idx - 1].is_ascii_alphabetic() {
                idx += 1;
                continue;
            }

            let mut bracket = idx;
            while bracket < range.end && bytes[bracket].is_ascii_alphabetic() {
                bracket += 1;
            }
            if bracket >= range.end || bytes[bracket] != b'[' {
                idx += 1;
                continue;
            }

            let Some(close_rel) = markdown[bracket + 1..range.end].find(']') else {
                idx += 1;
                continue;
            };
            let close = bracket + 1 + close_rel;
            if parse_req_marker(&markdown[bracket + 1..close]).is_err() {
                idx += 1;
                continue;
            }

            let original = markdown[idx..=close].to_string();
            bytes[idx] = b'~';
            let sanitized = String::from_utf8_lossy(&bytes[idx..=close]).into_owned();
            replacements.push((sanitized, original));
            idx = close + 1;
        }
    }

    SanitizedMarkdown {
        markdown: String::from_utf8(bytes).unwrap_or_else(|_| markdown.to_string()),
        replacements,
    }
}

fn markdown_link_ranges(markdown: &str) -> Vec<Range<usize>> {
    Parser::new_ext(markdown, Options::all())
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => Some(range),
            _ => None,
        })
        .collect()
}

pub(crate) fn restore_link_markers(doc: &mut Document, replacements: &[(String, String)]) {
    restore_text(&mut doc.html, replacements);
    for heading in &mut doc.headings {
        let original = heading.title.clone();
        restore_text(&mut heading.title, replacements);
        if heading.title != original {
            heading.id = marq::slugify(&heading.title);
        }
    }
    for req in &mut doc.reqs {
        restore_text(&mut req.raw, replacements);
        restore_text(&mut req.html, replacements);
    }
    for element in &mut doc.elements {
        match element {
            DocElement::Heading(heading) => {
                let original = heading.title.clone();
                restore_text(&mut heading.title, replacements);
                if heading.title != original {
                    heading.id = marq::slugify(&heading.title);
                }
            }
            DocElement::Req(req) => {
                restore_text(&mut req.raw, replacements);
                restore_text(&mut req.html, replacements);
            }
            DocElement::Paragraph(_) => {}
        }
    }
}

fn restore_text(text: &mut String, replacements: &[(String, String)]) {
    for (sanitized, original) in replacements {
        if text.contains(sanitized) {
            *text = text.replace(sanitized, original);
        }
    }
}
