use std::path::Path;

use comrak::{
    arena_tree::Node,
    nodes::{Ast, NodeValue},
    parse_document, Arena, Options,
};

/// Extract a project description from a README file.
///
/// Reads README.md (or README) from the project root, parses the markdown,
/// and extracts the first few sentences of prose text — skipping headings,
/// badges, images, and HTML blocks.
pub fn extract_readme_description(project_root: &Path) -> Option<String> {
    let readme_path = find_readme(project_root)?;
    let content = std::fs::read_to_string(&readme_path).ok()?;

    if content.trim().is_empty() {
        return None;
    }

    let description = extract_description_from_markdown(&content);
    if description.is_empty() {
        None
    } else {
        Some(description)
    }
}

/// Find the README file in a directory, trying common names.
fn find_readme(dir: &Path) -> Option<std::path::PathBuf> {
    let candidates = [
        "README.md",
        "readme.md",
        "Readme.md",
        "README.MD",
        "README",
        "readme",
        "README.rst",
        "README.txt",
    ];

    for name in &candidates {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Parse markdown and extract the first meaningful paragraph text.
/// Skips: headings, images, badges (links wrapping images), HTML blocks.
fn extract_description_from_markdown(markdown: &str) -> String {
    let arena = Arena::new();
    let options = Options::default();
    let root = parse_document(&arena, markdown, &options);

    let mut paragraphs: Vec<String> = Vec::new();
    let max_sentences = 3;
    let max_chars = 500;

    collect_paragraphs(root, &mut paragraphs);

    // Join paragraphs and truncate to max_sentences / max_chars
    let combined = paragraphs.join(" ");
    truncate_to_sentences(&combined, max_sentences, max_chars)
}

/// Recursively collect text from paragraph nodes, skipping images and badges.
fn collect_paragraphs<'a>(node: &'a Node<'a, std::cell::RefCell<Ast>>, out: &mut Vec<String>) {
    for child in node.children() {
        let ast = child.data.borrow();
        match &ast.value {
            NodeValue::Paragraph => {
                let text = extract_text_from_node(child);
                let trimmed = text.trim().to_string();
                // Skip empty paragraphs and badge lines (contain shield.io or badge URLs)
                if !trimmed.is_empty() && !is_badge_line(&trimmed) {
                    out.push(trimmed);
                }
            }
            // Skip headings, code blocks, HTML, images, tables at top level
            NodeValue::Heading(_)
            | NodeValue::CodeBlock(_)
            | NodeValue::HtmlBlock(_)
            | NodeValue::Table(_)
            | NodeValue::ThematicBreak => {}
            // Recurse into block containers like BlockQuote, List, etc.
            _ => collect_paragraphs(child, out),
        }
    }
}

/// Extract plain text from a node, skipping image nodes.
fn extract_text_from_node<'a>(node: &'a Node<'a, std::cell::RefCell<Ast>>) -> String {
    let mut text = String::new();

    for child in node.children() {
        let ast = child.data.borrow();
        match &ast.value {
            NodeValue::Text(t) => text.push_str(t),
            NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
            NodeValue::Code(c) => {
                text.push('`');
                text.push_str(&c.literal);
                text.push('`');
            }
            // Skip images entirely (badges, screenshots, etc.)
            NodeValue::Image(_) => {}
            // Recurse into inline formatting (bold, italic, links)
            _ => text.push_str(&extract_text_from_node(child)),
        }
    }

    text
}

/// Detect badge/shield lines (common in READMEs).
fn is_badge_line(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("shields.io")
        || lower.contains("badge")
        || lower.contains("img.shields")
        || lower.contains("[![")
        || (lower.contains("![") && lower.contains("](http"))
}

/// Truncate text to N sentences or max characters, whichever comes first.
fn truncate_to_sentences(text: &str, max_sentences: usize, max_chars: usize) -> String {
    let mut result = String::new();
    let mut sentence_count = 0;

    for ch in text.chars() {
        if result.len() >= max_chars {
            // Truncate at word boundary
            if let Some(pos) = result.rfind(' ') {
                result.truncate(pos);
            }
            result.push_str("...");
            break;
        }

        result.push(ch);

        if ch == '.' || ch == '!' || ch == '?' {
            sentence_count += 1;
            if sentence_count >= max_sentences {
                break;
            }
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_description_skips_badges() {
        let md = r#"# My Project

[![Build Status](https://shields.io/badge/build-passing)](https://ci.example.com)
[![License](https://img.shields.io/badge/license-MIT)](LICENSE)

A powerful tool for analyzing codebases. It supports multiple languages and produces high-quality output.

## Features

- Fast scanning
- Multi-language support
"#;
        let desc = extract_description_from_markdown(md);
        assert!(desc.starts_with("A powerful tool"));
        assert!(!desc.contains("shields.io"));
        assert!(!desc.contains("Build Status"));
    }

    #[test]
    fn test_truncate_to_sentences() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let result = truncate_to_sentences(text, 2, 500);
        assert_eq!(result, "First sentence. Second sentence.");
    }

    #[test]
    fn test_truncate_to_max_chars() {
        let text = "This is a very long sentence that goes on and on.";
        let result = truncate_to_sentences(text, 10, 30);
        assert!(result.len() <= 33); // 30 + "..."
    }
}
