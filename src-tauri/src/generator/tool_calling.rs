use serde_json::json;

use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;

use super::tools::readonly_tools;
use super::types::*;
use super::Generator;

/// Generates multi-step code assistant tool calling training data.
/// Creates realistic scenarios where a model uses tools like read_file,
/// search_code, edit_file to interact with the actual codebase.
pub struct ToolCallingGenerator;

impl Generator for ToolCallingGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::ToolCalling
    }

    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        for analysis in ctx.analyses {
            for symbol in &analysis.symbols {
                if !matches!(
                    symbol.kind,
                    SymbolKind::Function | SymbolKind::Method
                ) {
                    continue;
                }

                if symbol.source_text.lines().count() < 5 {
                    continue;
                }

                // Pattern 1: "Explain what X does" → read_file → explanation
                if let Some(sample) = generate_read_and_explain(ctx, analysis, symbol) {
                    samples.push(sample);
                }

                // Pattern 2: "Find where X is used" → search_code → summary
                if symbol.source_text.lines().count() > 8 {
                    if let Some(sample) = generate_find_references(ctx, analysis, symbol) {
                        samples.push(sample);
                    }
                }
            }

            // Pattern 3: "What files are in this directory?" → list_directory
            if !analysis.symbols.is_empty() {
                if let Some(sample) = generate_explore_structure(ctx, analysis) {
                    samples.push(sample);
                }
            }
        }

        samples
    }
}

/// Pattern: User asks about a function → assistant reads the file → explains it
fn generate_read_and_explain(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let file = &analysis.file_path;
    let _file_content = ctx.file_contents.get(file)?;

    let system_prompt = format!(
        "You are a coding assistant for the {} project. \
         Use the provided tools to read files and answer questions about the codebase.",
        ctx.project_name
    );

    let user_msg = format!(
        "Can you explain what the `{}` {} does in this project?",
        symbol.name, symbol.kind
    );

    // Assistant decides to read the file
    let read_call = ToolCall::new(
        "read_file",
        json!({
            "path": file,
            "start_line": symbol.span.start_line,
            "end_line": symbol.span.end_line
        }),
    );

    // Simulate tool result with actual code
    let tool_result = symbol.source_text.clone();

    // Assistant explains based on the code
    let explanation = build_explanation(symbol, analysis);

    let tools = readonly_tools();

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::ToolCalling,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_msg),
            ConversationTurn::assistant_with_tool_calls(
                format!("Let me read the source code for `{}`.", symbol.name),
                vec![read_call.clone()],
            ),
            ConversationTurn::tool_result(&read_call.id, tool_result),
            ConversationTurn::assistant(explanation),
        ],
        tools: Some(tools),
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: Some(symbol.name.clone()),
            symbol_kind: Some(symbol.kind.to_string()),
            estimated_tokens: symbol.source_text.len() / 4 + 200,
            complexity_tier: if symbol.source_text.lines().count() > 30 {
                ComplexityTier::Complex
            } else {
                ComplexityTier::Moderate
            },
        },
    })
}

/// Pattern: User asks where a function is used → assistant searches → summarizes
fn generate_find_references(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let file = &analysis.file_path;

    // Find other files that import this symbol
    let referencing_files: Vec<&str> = ctx
        .analyses
        .iter()
        .filter(|a| a.file_path != *file)
        .filter(|a| {
            a.imports
                .iter()
                .any(|imp| imp.source.contains(&symbol.name))
                || a.symbols
                    .iter()
                    .any(|s| s.source_text.contains(&symbol.name))
        })
        .map(|a| a.file_path.as_str())
        .take(3)
        .collect();

    let system_prompt = format!(
        "You are a coding assistant for the {} project. \
         Use tools to search the codebase and answer questions.",
        ctx.project_name
    );

    let user_msg = format!(
        "Where is `{}` used in the project?",
        symbol.name
    );

    let search_call = ToolCall::new(
        "search_code",
        json!({
            "pattern": symbol.name,
        }),
    );

    // Build simulated search results
    let mut search_result = String::new();
    search_result.push_str(&format!("{}:{}  (definition)\n", file, symbol.span.start_line));
    for rf in &referencing_files {
        search_result.push_str(&format!("{rf}:  (usage)\n"));
    }
    if referencing_files.is_empty() {
        search_result.push_str("No other references found in the scanned files.\n");
    }

    let summary = if referencing_files.is_empty() {
        format!(
            "`{}` is defined in `{}` at line {}. \
             It doesn't appear to be referenced from other files in the current scope.",
            symbol.name, file, symbol.span.start_line
        )
    } else {
        format!(
            "`{}` is defined in `{}` at line {} and is used in {} other file(s): {}.",
            symbol.name,
            file,
            symbol.span.start_line,
            referencing_files.len(),
            referencing_files.join(", ")
        )
    };

    let tools = readonly_tools();

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::ToolCalling,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_msg),
            ConversationTurn::assistant_with_tool_calls(
                format!("Let me search for references to `{}`.", symbol.name),
                vec![search_call.clone()],
            ),
            ConversationTurn::tool_result(&search_call.id, search_result),
            ConversationTurn::assistant(summary),
        ],
        tools: Some(tools),
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: Some(symbol.name.clone()),
            symbol_kind: Some(symbol.kind.to_string()),
            estimated_tokens: 300,
            complexity_tier: ComplexityTier::Moderate,
        },
    })
}

/// Pattern: User asks about project structure → list_directory → overview
fn generate_explore_structure(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
) -> Option<TrainingSample> {
    let file = &analysis.file_path;

    // Get the directory of this file
    let dir = std::path::Path::new(file)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    // Collect other files in the same directory
    let sibling_files: Vec<&str> = ctx
        .analyses
        .iter()
        .filter(|a| {
            std::path::Path::new(&a.file_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
                == dir
        })
        .map(|a| a.file_path.as_str())
        .take(10)
        .collect();

    if sibling_files.len() < 2 {
        return None;
    }

    let system_prompt = format!(
        "You are a coding assistant for the {} project. \
         Use tools to explore the codebase structure.",
        ctx.project_name
    );

    let user_msg = format!("What files are in the `{dir}` directory?");

    let list_call = ToolCall::new("list_directory", json!({ "path": dir }));

    let listing = sibling_files.join("\n");

    let summary = format!(
        "The `{dir}` directory contains {} file(s):\n\n{}",
        sibling_files.len(),
        sibling_files
            .iter()
            .map(|f| format!("- `{f}`"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let tools = readonly_tools();

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: sibling_files.iter().map(|s| s.to_string()).collect(),
        sample_type: SampleType::ToolCalling,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_msg),
            ConversationTurn::assistant_with_tool_calls(
                format!("Let me list the contents of `{dir}`."),
                vec![list_call.clone()],
            ),
            ConversationTurn::tool_result(&list_call.id, listing),
            ConversationTurn::assistant(summary),
        ],
        tools: Some(tools),
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: None,
            symbol_kind: None,
            estimated_tokens: 250,
            complexity_tier: ComplexityTier::Simple,
        },
    })
}

fn build_explanation(symbol: &Symbol, analysis: &CodeAnalysis) -> String {
    let mut explanation = String::new();

    if let Some(doc) = &symbol.doc_comment {
        explanation.push_str(&format!(
            "The `{}` {} in `{}` {}.\n\n",
            symbol.name, symbol.kind, analysis.file_path, doc
        ));
    } else {
        explanation.push_str(&format!(
            "The `{}` {} is defined in `{}`.\n\n",
            symbol.name, symbol.kind, analysis.file_path
        ));
    }

    if !symbol.parameters.is_empty() {
        explanation.push_str("**Parameters:**\n");
        for p in &symbol.parameters {
            let ty = p.type_annotation.as_deref().unwrap_or("unspecified");
            explanation.push_str(&format!("- `{}`: {ty}\n", p.name));
        }
        explanation.push('\n');
    }

    if let Some(ret) = &symbol.return_type {
        explanation.push_str(&format!("**Returns:** `{ret}`\n"));
    }

    explanation
}
