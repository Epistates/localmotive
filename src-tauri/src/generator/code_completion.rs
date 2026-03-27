use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;

use super::types::*;
use super::Generator;

/// Generates fill-in-the-middle (FIM) training data using AST-aware span selection.
/// Selects meaningful code boundaries (function bodies, conditionals, loop bodies)
/// rather than random character offsets.
pub struct CodeCompletionGenerator;

impl Generator for CodeCompletionGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::CodeCompletion
    }

    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        for analysis in ctx.analyses {
            let source = match ctx.file_contents.get(&analysis.file_path) {
                Some(s) => s,
                None => continue,
            };

            for symbol in &analysis.symbols {
                if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
                    continue;
                }

                let Some(body) = &symbol.body_text else {
                    continue;
                };

                // Skip trivially small bodies
                if body.lines().count() < 3 {
                    continue;
                }

                // Generate FIM at the function body level
                if let Some(sample) = generate_fim_body(ctx, analysis, symbol, source) {
                    samples.push(sample);
                }
            }
        }

        samples
    }
}

/// Generate a FIM sample where the function body is the "middle" to fill in.
/// The prefix is everything up to and including the function signature,
/// the suffix is everything after the function, and the middle is the body.
fn generate_fim_body(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
    source: &str,
) -> Option<TrainingSample> {
    let body = symbol.body_text.as_ref()?;

    // The source_text includes the full function definition
    let func_text = &symbol.source_text;

    // Find where the body starts within the function text
    let body_start_in_func = func_text.find(body.as_str())?;

    // Prefix: file content up to function + signature (before body)
    let prefix_end = symbol.span.byte_start + body_start_in_func;
    if prefix_end > source.len() {
        return None;
    }
    let prefix = &source[..prefix_end];

    // Middle: the function body
    let middle = body;

    // Suffix: everything after the function
    let suffix_start = symbol.span.byte_end;
    if suffix_start > source.len() {
        return None;
    }
    let suffix = &source[suffix_start..];

    // Trim to reasonable size
    let prefix_trimmed = trim_prefix(prefix, 50);
    let suffix_trimmed = trim_suffix(suffix, 20);

    let lang = lang_code(&analysis.language);

    let system_prompt = format!(
        "You are an expert {} developer. Complete the code by filling in the missing function body.",
        analysis.language
    );

    let question = format!(
        "Complete the function body for the following code. \
         The `<FILL>` marker shows where the code should be inserted.\n\n\
         ```{lang}\n{prefix_trimmed}<FILL>{suffix_trimmed}\n```"
    );

    let answer = format!("```{lang}\n{middle}\n```");

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![analysis.file_path.clone()],
        sample_type: SampleType::CodeCompletion,
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
            estimated_tokens: (prefix_trimmed.len() + middle.len() + suffix_trimmed.len()) / 4,
            complexity_tier: if middle.lines().count() > 20 {
                ComplexityTier::Complex
            } else if middle.lines().count() > 8 {
                ComplexityTier::Moderate
            } else {
                ComplexityTier::Simple
            },
        },
    })
}

/// Keep the last N lines of the prefix for context.
fn trim_prefix(prefix: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = prefix.lines().collect();
    if lines.len() <= max_lines {
        return prefix.to_string();
    }
    let start = lines.len() - max_lines;
    lines[start..].join("\n")
}

/// Keep the first N lines of the suffix for context.
fn trim_suffix(suffix: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = suffix.lines().collect();
    if lines.len() <= max_lines {
        return suffix.to_string();
    }
    lines[..max_lines].join("\n")
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
