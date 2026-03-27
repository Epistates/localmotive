use serde::{Deserialize, Serialize};
use tree_sitter::Node;

/// Complexity metrics for a code region.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexityMetrics {
    /// McCabe cyclomatic complexity (branches + 1).
    pub cyclomatic: u32,
    /// Maximum nesting depth of control structures.
    pub max_nesting: u32,
    /// Number of lines in the function body.
    pub line_count: u32,
    /// Number of parameters.
    pub parameter_count: u32,
}

const MAX_AST_DEPTH: u32 = 256;

/// Compute complexity metrics for a tree-sitter subtree (e.g., a function body).
pub fn compute_complexity(node: Node) -> ComplexityMetrics {
    let mut cyclomatic = 1u32; // Start at 1 (base path)
    let mut max_nesting = 0u32;
    let mut current_nesting = 0u32;

    walk_for_complexity(node, &mut cyclomatic, &mut max_nesting, &mut current_nesting, 0);

    let start_line = node.start_position().row;
    let end_line = node.end_position().row;
    let line_count = (end_line.saturating_sub(start_line) + 1) as u32;

    ComplexityMetrics {
        cyclomatic,
        max_nesting,
        line_count,
        parameter_count: 0, // Set by caller from Symbol.parameters.len()
    }
}

fn walk_for_complexity(
    node: Node,
    cyclomatic: &mut u32,
    max_nesting: &mut u32,
    current_nesting: &mut u32,
    depth: u32,
) {
    if depth > MAX_AST_DEPTH {
        return;
    }
    let kind = node.kind();

    // Branch nodes that increase cyclomatic complexity
    let is_branch = matches!(
        kind,
        "if_expression"
            | "if_statement"
            | "if_let_expression"
            | "else_clause"
            | "elif_clause"
            | "for_statement"
            | "for_expression"
            | "while_statement"
            | "while_expression"
            | "while_let_expression"
            | "loop_expression"
            | "match_expression"
            | "switch_statement"
            | "switch_expression"
            | "case_clause"
            | "match_arm"
            | "catch_clause"
            | "ternary_expression"
            | "conditional_expression"
            | "binary_expression" // && and || add paths
            | "logical_and"
            | "logical_or"
            | "try_statement"
    );

    // For binary_expression, only count && and ||
    if kind == "binary_expression" {
        let op = node.child_by_field_name("operator");
        if let Some(op) = op {
            let op_text = op.kind();
            if op_text == "&&" || op_text == "||" {
                *cyclomatic += 1;
            }
        }
    } else if is_branch {
        *cyclomatic += 1;
    }

    // Nesting depth tracking
    let increases_nesting = matches!(
        kind,
        "if_expression"
            | "if_statement"
            | "if_let_expression"
            | "for_statement"
            | "for_expression"
            | "while_statement"
            | "while_expression"
            | "while_let_expression"
            | "loop_expression"
            | "match_expression"
            | "switch_statement"
            | "try_statement"
            | "catch_clause"
            | "block" // Only block-level nesting
    );

    if increases_nesting {
        *current_nesting += 1;
        if *current_nesting > *max_nesting {
            *max_nesting = *current_nesting;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_complexity(child, cyclomatic, max_nesting, current_nesting, depth + 1);
    }

    if increases_nesting {
        *current_nesting -= 1;
    }
}
