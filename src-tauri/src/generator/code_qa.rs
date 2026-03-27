use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;

use super::types::*;
use super::Generator;

/// Generates "What does this code do?" Q&A training pairs.
pub struct CodeQAGenerator;

impl Generator for CodeQAGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::CodeQA
    }

    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        for analysis in ctx.analyses {
            for symbol in &analysis.symbols {
                // Only generate Q&A for functions, methods, classes, structs, traits
                if !is_qa_worthy(symbol) {
                    continue;
                }

                // Skip very short symbols (getters, simple accessors)
                if symbol.source_text.lines().count() < 4 {
                    continue;
                }

                if let Some(sample) = generate_explanation_qa(ctx, analysis, symbol) {
                    samples.push(sample);
                }

                // For complex symbols, also generate a "how would you use this?" pair
                if symbol.source_text.lines().count() > 15 {
                    if let Some(sample) = generate_usage_qa(ctx, analysis, symbol) {
                        samples.push(sample);
                    }
                }
            }
        }

        samples
    }
}

fn is_qa_worthy(symbol: &Symbol) -> bool {
    matches!(
        symbol.kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Class
            | SymbolKind::Struct
            | SymbolKind::Trait
            | SymbolKind::Interface
            | SymbolKind::Enum
    )
}

/// Generate: "What does X do?" → detailed explanation
fn generate_explanation_qa(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let kind_str = symbol.kind.to_string();
    let file = &analysis.file_path;
    let lang = &analysis.language;

    let system_prompt = format!(
        "You are an expert {lang} developer working on the {} project. \
         {}Explain code clearly and concisely.",
        ctx.project_name,
        if ctx.project_description.is_empty() {
            String::new()
        } else {
            format!("Project description: {}. ", ctx.project_description)
        },
    );

    let question = match symbol.kind {
        SymbolKind::Function | SymbolKind::Method => {
            format!(
                "What does the `{}` {} in `{}` do?\n\n```{}\n{}\n```",
                symbol.name,
                kind_str,
                file,
                lang_id(lang),
                symbol.source_text
            )
        }
        SymbolKind::Class | SymbolKind::Struct | SymbolKind::Trait | SymbolKind::Interface => {
            format!(
                "Explain the `{}` {} defined in `{}`.\n\n```{}\n{}\n```",
                symbol.name,
                kind_str,
                file,
                lang_id(lang),
                symbol.source_text
            )
        }
        SymbolKind::Enum => {
            format!(
                "What is the purpose of the `{}` enum in `{}`?\n\n```{}\n{}\n```",
                symbol.name,
                file,
                lang_id(lang),
                symbol.source_text
            )
        }
        _ => return None,
    };

    let answer = generate_explanation(symbol, analysis);

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::CodeQA,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(question),
            ConversationTurn::assistant(answer),
        ],
        tools: None,
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: Some(symbol.name.clone()),
            symbol_kind: Some(kind_str),
            estimated_tokens: estimate_tokens(&symbol.source_text),
            complexity_tier: complexity_tier(symbol),
        },
    })
}

/// Generate: "How would you use X?" → usage example
fn generate_usage_qa(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    if !matches!(
        symbol.kind,
        SymbolKind::Function | SymbolKind::Method | SymbolKind::Class | SymbolKind::Struct
    ) {
        return None;
    }

    let lang = &analysis.language;
    let file = &analysis.file_path;

    let system_prompt = format!(
        "You are an expert {lang} developer. Provide clear, practical code examples.",
    );

    let question = format!(
        "How would you use the `{}` {} from `{}`? Show a practical example.\n\n```{}\n{}\n```",
        symbol.name,
        symbol.kind,
        file,
        lang_id(lang),
        symbol.signature.as_deref().unwrap_or(&symbol.source_text)
    );

    let answer = generate_usage_example(symbol, analysis);

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::CodeQA,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(question),
            ConversationTurn::assistant(answer),
        ],
        tools: None,
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: Some(symbol.name.clone()),
            symbol_kind: Some(symbol.kind.to_string()),
            estimated_tokens: estimate_tokens(&symbol.source_text),
            complexity_tier: complexity_tier(symbol),
        },
    })
}

/// Template-based explanation generation from code structure.
fn generate_explanation(symbol: &Symbol, analysis: &CodeAnalysis) -> String {
    let mut parts = Vec::new();

    // Opening statement
    match symbol.kind {
        SymbolKind::Function | SymbolKind::Method => {
            let parent_ctx = symbol
                .parent
                .as_ref()
                .map(|p| format!(" on `{p}`"))
                .unwrap_or_default();
            parts.push(format!(
                "The `{}` {}{} in `{}` ",
                symbol.name,
                symbol.kind,
                parent_ctx,
                analysis.file_path,
            ));

            // Use doc comment if available
            if let Some(doc) = &symbol.doc_comment {
                parts.push(format!("{}.", doc.trim_end_matches('.')));
            } else {
                // Infer purpose from name
                parts.push(format!("{}.", infer_purpose_from_name(&symbol.name)));
            }
        }
        SymbolKind::Class | SymbolKind::Struct => {
            parts.push(format!(
                "`{}` is a {} defined in `{}`. ",
                symbol.name, symbol.kind, analysis.file_path,
            ));
            if let Some(doc) = &symbol.doc_comment {
                parts.push(format!("{}.", doc.trim_end_matches('.')));
            }
        }
        SymbolKind::Trait | SymbolKind::Interface => {
            parts.push(format!(
                "`{}` is a {} that defines a contract in `{}`. ",
                symbol.name, symbol.kind, analysis.file_path,
            ));
            if let Some(doc) = &symbol.doc_comment {
                parts.push(format!("{}.", doc.trim_end_matches('.')));
            }
        }
        SymbolKind::Enum => {
            parts.push(format!(
                "`{}` is an enum in `{}` that represents a set of distinct variants. ",
                symbol.name, analysis.file_path,
            ));
            if let Some(doc) = &symbol.doc_comment {
                parts.push(format!("{}.", doc.trim_end_matches('.')));
            }
        }
        _ => {}
    }

    // Parameters
    if !symbol.parameters.is_empty() {
        let params_desc: Vec<String> = symbol
            .parameters
            .iter()
            .map(|p| {
                if let Some(ty) = &p.type_annotation {
                    format!("`{}` ({})", p.name, ty)
                } else {
                    format!("`{}`", p.name)
                }
            })
            .collect();
        parts.push(format!("\n\nIt takes {} parameter(s): {}.", params_desc.len(), params_desc.join(", ")));
    }

    // Return type
    if let Some(ret) = &symbol.return_type {
        parts.push(format!(" It returns `{ret}`."));
    }

    parts.join("")
}

fn generate_usage_example(symbol: &Symbol, analysis: &CodeAnalysis) -> String {
    let lang = lang_id(&analysis.language);
    let mut example = format!(
        "Here's how you might use `{}` from `{}`:\n\n```{lang}\n",
        symbol.name, analysis.file_path
    );

    match symbol.kind {
        SymbolKind::Function => {
            let args: Vec<&str> = symbol
                .parameters
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            example.push_str(&format!(
                "let result = {}({});\n",
                symbol.name,
                args.join(", ")
            ));
        }
        SymbolKind::Method => {
            if let Some(parent) = &symbol.parent {
                let args: Vec<&str> = symbol
                    .parameters
                    .iter()
                    .filter(|p| p.name != "self" && p.name != "&self" && p.name != "&mut self")
                    .map(|p| p.name.as_str())
                    .collect();
                example.push_str(&format!(
                    "let instance = {parent}::new();\ninstance.{}({});\n",
                    symbol.name,
                    args.join(", ")
                ));
            }
        }
        SymbolKind::Struct | SymbolKind::Class => {
            example.push_str(&format!("let instance = {}::new();\n", symbol.name));
        }
        _ => {
            example.push_str(&format!("// Use {} as needed\n", symbol.name));
        }
    }

    example.push_str("```");
    example
}

fn infer_purpose_from_name(name: &str) -> String {
    let words: Vec<&str> = split_camel_snake(name);
    if words.is_empty() {
        return "performs an operation".to_string();
    }

    let verb = words[0].to_lowercase();
    let rest: String = words[1..].iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join(" ");

    match verb.as_str() {
        "get" | "fetch" | "load" | "read" | "find" | "query" | "retrieve" => {
            format!("retrieves {rest}")
        }
        "set" | "update" | "put" | "store" | "save" | "write" => {
            format!("updates {rest}")
        }
        "create" | "new" | "build" | "make" | "init" | "initialize" => {
            format!("creates a new {rest}")
        }
        "delete" | "remove" | "drop" | "clear" | "destroy" => {
            format!("removes {rest}")
        }
        "check" | "is" | "has" | "can" | "should" | "validate" | "verify" => {
            format!("checks whether {rest}")
        }
        "parse" | "decode" | "deserialize" => {
            format!("parses {rest}")
        }
        "format" | "render" | "display" | "serialize" | "encode" => {
            format!("formats {rest}")
        }
        "handle" | "process" | "run" | "execute" | "perform" => {
            format!("handles {rest}")
        }
        "convert" | "transform" | "map" | "to" | "from" | "into" => {
            format!("converts {rest}")
        }
        "sort" | "filter" | "group" | "merge" | "split" => {
            format!("{verb}s {rest}")
        }
        "test" => format!("tests {rest}"),
        _ => format!("{verb}s {rest}"),
    }
}

fn split_camel_snake(name: &str) -> Vec<&str> {
    // Split on underscores first
    if name.contains('_') {
        return name.split('_').filter(|s| !s.is_empty()).collect();
    }
    // Simple split for camelCase — return the whole name as one word
    // (full camelCase splitting is complex, this is good enough for inference)
    vec![name]
}

fn lang_id(lang: &crate::scanner::Language) -> &'static str {
    use crate::scanner::Language;
    match lang {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::JavaScript => "javascript",
        Language::Python => "python",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::CSharp => "csharp",
        Language::Ruby => "ruby",
        Language::Swift => "swift",
        Language::Kotlin => "kotlin",
        Language::Shell => "bash",
        Language::Lua => "lua",
        _ => "",
    }
}

fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for code
    text.len() / 4
}

fn complexity_tier(symbol: &Symbol) -> ComplexityTier {
    let lines = symbol.source_text.lines().count();
    if lines < 10 {
        ComplexityTier::Simple
    } else if lines < 40 {
        ComplexityTier::Moderate
    } else {
        ComplexityTier::Complex
    }
}
