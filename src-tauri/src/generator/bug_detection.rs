use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;
use crate::scanner::Language;

use super::types::*;
use super::Generator;

/// Generates bug detection training data.
/// Presents code and asks the model to identify potential issues.
pub struct BugDetectionGenerator;

impl Generator for BugDetectionGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::BugDetection
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

                if symbol.source_text.lines().count() < 8 {
                    continue;
                }

                let issues = detect_potential_issues(symbol, &analysis.language);
                if !issues.is_empty() {
                    if let Some(sample) =
                        generate_bug_detection(ctx, analysis, symbol, &issues)
                    {
                        samples.push(sample);
                    }
                }
            }
        }

        samples
    }
}

/// Heuristic: detect common patterns that might indicate issues.
fn detect_potential_issues(symbol: &Symbol, language: &Language) -> Vec<String> {
    let code = &symbol.source_text;
    let mut issues = Vec::new();

    // Language-specific patterns
    match language {
        Language::Rust => {
            if code.contains(".unwrap()") {
                issues.push(
                    "Uses `.unwrap()` which will panic on `None`/`Err`. Consider using \
                     `.expect()` with a message or proper error handling with `?`."
                        .to_string(),
                );
            }
            if code.contains("unsafe {") || code.contains("unsafe{") {
                issues.push(
                    "Contains `unsafe` block. Ensure memory safety invariants are \
                     upheld and document why `unsafe` is necessary."
                        .to_string(),
                );
            }
            if code.contains(".clone()") && code.matches(".clone()").count() > 2 {
                issues.push(
                    "Multiple `.clone()` calls detected. Consider whether ownership can \
                     be restructured to avoid unnecessary cloning."
                        .to_string(),
                );
            }
        }
        Language::TypeScript | Language::JavaScript => {
            if code.contains("any") && code.contains(": any") {
                issues.push(
                    "Uses `any` type annotation. This bypasses TypeScript's type checking. \
                     Consider using a more specific type or `unknown`."
                        .to_string(),
                );
            }
            if code.contains("== ") && !code.contains("=== ") {
                issues.push(
                    "Uses loose equality (`==`) which performs type coercion. \
                     Prefer strict equality (`===`)."
                        .to_string(),
                );
            }
            if code.contains("console.log") {
                issues.push(
                    "Contains `console.log` which should be removed before production."
                        .to_string(),
                );
            }
        }
        Language::Python => {
            if code.contains("except:") || code.contains("except Exception:") {
                issues.push(
                    "Catches broad exceptions. Consider catching specific exception types \
                     to avoid masking unexpected errors."
                        .to_string(),
                );
            }
            if code.contains("eval(") || code.contains("exec(") {
                issues.push(
                    "Uses `eval()`/`exec()` which can execute arbitrary code. \
                     This is a security risk if input is not sanitized."
                        .to_string(),
                );
            }
        }
        Language::Go => {
            if code.contains("_ = err") || code.contains("_ =err") {
                issues.push(
                    "Ignores an error value. All errors should be handled or explicitly \
                     documented as intentionally ignored."
                        .to_string(),
                );
            }
        }
        _ => {}
    }

    // Cross-language patterns
    if code.contains("TODO") || code.contains("FIXME") || code.contains("HACK") {
        issues.push(
            "Contains TODO/FIXME/HACK comments indicating incomplete or \
             temporary code that should be addressed."
                .to_string(),
        );
    }

    // No error handling in function with potential failure points
    if symbol.parameters.len() > 0
        && !code.contains("Error")
        && !code.contains("error")
        && !code.contains("Result")
        && !code.contains("try")
        && !code.contains("catch")
        && !code.contains("except")
        && symbol.source_text.lines().count() > 15
    {
        issues.push(
            "No visible error handling in a substantial function. Consider adding \
             input validation and error handling for robustness."
                .to_string(),
        );
    }

    issues
}

fn generate_bug_detection(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
    issues: &[String],
) -> Option<TrainingSample> {
    let file = &analysis.file_path;
    let lang = &analysis.language;
    let lang_str = lang_code(lang);

    let system_prompt = format!(
        "You are an expert {lang} developer specializing in code quality and bug detection. \
         Identify potential issues, bugs, and improvements in the code."
    );

    let question = format!(
        "Review this {} for potential issues:\n\n```{lang_str}\n{}\n```",
        symbol.kind, symbol.source_text
    );

    let mut answer = format!("I found {} potential issue(s) in `{}`:\n\n", issues.len(), symbol.name);
    for (i, issue) in issues.iter().enumerate() {
        answer.push_str(&format!("{}. {issue}\n\n", i + 1));
    }

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::BugDetection,
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
            estimated_tokens: symbol.source_text.len() / 4 + 200,
            complexity_tier: ComplexityTier::Moderate,
        },
    })
}

fn lang_code(lang: &Language) -> &'static str {
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
