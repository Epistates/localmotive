pub mod ast_extraction;
pub mod complexity;
pub mod symbol_table;
pub mod tree_sitter;

use crate::error::Result;
use crate::scanner::FileInfo;

pub use ast_extraction::{CodeAnalysis, Symbol, SymbolKind, Span, Parameter, Visibility, Import};
pub use complexity::ComplexityMetrics;
pub use symbol_table::{SymbolTable, build_symbol_table};
pub use self::tree_sitter::registry;

/// Analyze a single source file: parse AST, extract symbols, compute complexity.
pub fn analyze_file(file: &FileInfo) -> Result<Option<CodeAnalysis>> {
    let Some(ref language) = file.language else {
        return Ok(None);
    };

    let reg = registry();
    let Some(mut parser) = reg.parser_for(language) else {
        return Ok(None);
    };

    let source = std::fs::read_to_string(&file.path)?;
    let Some(tree) = parser.parse(&source, None) else {
        return Err(crate::error::Error::ParseError {
            file: file.relative_path.clone(),
            message: "tree-sitter failed to parse file".to_string(),
        });
    };

    let mut analysis = ast_extraction::extract_symbols(&source, &tree, language);
    analysis.file_path = file.relative_path.clone();

    // Compute complexity for each function/method symbol
    for symbol in &mut analysis.symbols {
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            // Re-parse to get node for complexity (we need the tree)
            // Use byte range to find the node
            let node = find_node_at_range(
                tree.root_node(),
                symbol.span.byte_start,
                symbol.span.byte_end,
            );
            if let Some(node) = node {
                let _metrics = complexity::compute_complexity(node);
                // Metrics are available for future use in generation quality scoring
            }
        }
    }

    Ok(Some(analysis))
}

/// Find the smallest node that covers a byte range.
fn find_node_at_range(
    node: ::tree_sitter::Node,
    start: usize,
    end: usize,
) -> Option<::tree_sitter::Node> {
    if node.start_byte() == start && node.end_byte() == end {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.start_byte() <= start && child.end_byte() >= end {
            return find_node_at_range(child, start, end);
        }
    }
    None
}
