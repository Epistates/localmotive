use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;

use super::types::*;
use super::Generator;

/// Generates documentation/docstring training pairs.
/// Targets undocumented functions and produces "write documentation for this" examples.
pub struct DocumentationGenerator;

impl Generator for DocumentationGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::Documentation
    }

    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        for analysis in ctx.analyses {
            for symbol in &analysis.symbols {
                if !matches!(
                    symbol.kind,
                    SymbolKind::Function | SymbolKind::Method | SymbolKind::Class | SymbolKind::Struct
                ) {
                    continue;
                }

                // Skip very short symbols
                if symbol.source_text.lines().count() < 3 {
                    continue;
                }

                if symbol.doc_comment.is_none() {
                    // Undocumented: generate "write docs for this" pair
                    if let Some(sample) = generate_write_docs(ctx, analysis, symbol) {
                        samples.push(sample);
                    }
                } else {
                    // Already documented: generate "improve this documentation" pair
                    if symbol.source_text.lines().count() > 10 {
                        if let Some(sample) = generate_improve_docs(ctx, analysis, symbol) {
                            samples.push(sample);
                        }
                    }
                }
            }
        }

        samples
    }
}

fn generate_write_docs(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let lang = &analysis.language;
    let file = &analysis.file_path;

    let system_prompt = format!(
        "You are an expert {lang} developer. Write clear, comprehensive documentation \
         following {lang} conventions.",
    );

    let question = format!(
        "Write documentation for the following {} in `{}`:\n\n```{}\n{}\n```",
        symbol.kind,
        file,
        lang_code(lang),
        symbol.source_text
    );

    let answer = generate_doc_comment(symbol, lang);

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::Documentation,
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
            estimated_tokens: symbol.source_text.len() / 4,
            complexity_tier: if symbol.source_text.lines().count() > 30 {
                ComplexityTier::Complex
            } else {
                ComplexityTier::Moderate
            },
        },
    })
}

fn generate_improve_docs(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let lang = &analysis.language;
    let file = &analysis.file_path;
    let existing_doc = symbol.doc_comment.as_deref()?;

    let system_prompt = format!(
        "You are an expert {lang} developer. Improve documentation to be more \
         comprehensive while keeping it concise.",
    );

    let question = format!(
        "Improve the documentation for this {} in `{}`.\n\n\
         Current documentation:\n{}\n\n\
         Code:\n```{}\n{}\n```",
        symbol.kind,
        file,
        existing_doc,
        lang_code(lang),
        symbol.source_text
    );

    let answer = format!(
        "Here's improved documentation for `{}`:\n\n{}",
        symbol.name,
        generate_doc_comment(symbol, lang)
    );

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::Documentation,
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
            estimated_tokens: symbol.source_text.len() / 4,
            complexity_tier: ComplexityTier::Moderate,
        },
    })
}

/// Generate a documentation comment in the style of the language.
fn generate_doc_comment(symbol: &Symbol, lang: &crate::scanner::Language) -> String {
    use crate::scanner::Language;

    let mut doc = String::new();

    let description = symbol
        .doc_comment
        .clone()
        .unwrap_or_else(|| infer_purpose(&symbol.name, &symbol.kind));

    match lang {
        Language::Rust => {
            doc.push_str(&format!("/// {description}\n"));
            if !symbol.parameters.is_empty() {
                doc.push_str("///\n/// # Arguments\n///\n");
                for p in &symbol.parameters {
                    if p.name == "self" || p.name == "&self" || p.name == "&mut self" {
                        continue;
                    }
                    let ty = p
                        .type_annotation
                        .as_deref()
                        .unwrap_or("unspecified type");
                    doc.push_str(&format!("/// * `{}` - {ty} parameter\n", p.name));
                }
            }
            if let Some(ret) = &symbol.return_type {
                doc.push_str(&format!("///\n/// # Returns\n///\n/// {ret}\n"));
            }
        }
        Language::Python => {
            doc.push_str(&format!("    \"\"\"{description}\n\n"));
            if !symbol.parameters.is_empty() {
                doc.push_str("    Args:\n");
                for p in &symbol.parameters {
                    let ty = p.type_annotation.as_deref().unwrap_or("Any");
                    doc.push_str(&format!("        {} ({}): Description.\n", p.name, ty));
                }
            }
            if let Some(ret) = &symbol.return_type {
                doc.push_str(&format!("\n    Returns:\n        {ret}\n"));
            }
            doc.push_str("    \"\"\"");
        }
        Language::TypeScript | Language::JavaScript | Language::Java => {
            doc.push_str(&format!("/**\n * {description}\n"));
            for p in &symbol.parameters {
                let ty = p.type_annotation.as_deref().unwrap_or("*");
                doc.push_str(&format!(
                    " * @param {{{}}} {} - Description\n",
                    ty, p.name
                ));
            }
            if let Some(ret) = &symbol.return_type {
                doc.push_str(&format!(" * @returns {{{ret}}}\n"));
            }
            doc.push_str(" */");
        }
        Language::Go => {
            doc.push_str(&format!("// {} {description}", symbol.name));
        }
        _ => {
            doc.push_str(&format!("// {description}"));
        }
    }

    doc
}

fn infer_purpose(name: &str, kind: &SymbolKind) -> String {
    let action = match kind {
        SymbolKind::Function | SymbolKind::Method => "performs an operation",
        SymbolKind::Class | SymbolKind::Struct => "represents a data structure",
        SymbolKind::Trait | SymbolKind::Interface => "defines a contract",
        SymbolKind::Enum => "represents a set of variants",
        _ => "is a code element",
    };
    format!("`{name}` {action}")
}

fn lang_code(lang: &crate::scanner::Language) -> &'static str {
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
        _ => "",
    }
}
