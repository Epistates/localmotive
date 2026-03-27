use crate::analyzer::{CodeAnalysis, Symbol, SymbolKind};
use crate::pipeline::config::SampleType;

use super::types::*;
use super::Generator;

/// Generates multi-turn code review and debugging dialogue training data.
pub struct MultiTurnGenerator;

impl Generator for MultiTurnGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::MultiTurn
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

                // Only generate multi-turn for substantial functions
                if symbol.source_text.lines().count() < 10 {
                    continue;
                }

                if let Some(sample) = generate_code_review(ctx, analysis, symbol) {
                    samples.push(sample);
                }
            }
        }

        samples
    }
}

/// Generate a multi-turn code review dialogue.
fn generate_code_review(
    ctx: &GenerationContext,
    analysis: &CodeAnalysis,
    symbol: &Symbol,
) -> Option<TrainingSample> {
    let lang = &analysis.language;
    let file = &analysis.file_path;
    let lang_str = lang_code(lang);
    let lines = symbol.source_text.lines().count();

    let system_prompt = format!(
        "You are a senior {lang} developer conducting a code review. \
         Be constructive, specific, and suggest improvements where appropriate.",
    );

    // Turn 1: User submits code for review
    let user_1 = format!(
        "Can you review this {} `{}`?\n\n```{lang_str}\n{}\n```",
        symbol.kind, symbol.name, symbol.source_text
    );

    // Turn 2: Initial review feedback
    let mut review_points = Vec::new();

    // Check for documentation
    if symbol.doc_comment.is_none() {
        review_points.push(format!(
            "The {} lacks documentation. Adding a doc comment explaining its purpose, \
             parameters, and return value would improve maintainability.",
            symbol.kind
        ));
    }

    // Check function length
    if lines > 30 {
        review_points.push(format!(
            "At {} lines, this function is fairly long. Consider extracting some logic \
             into helper functions to improve readability.",
            lines
        ));
    }

    // Check parameter count
    if symbol.parameters.len() > 4 {
        review_points.push(format!(
            "This function takes {} parameters. Consider grouping related parameters \
             into a struct or options object to simplify the interface.",
            symbol.parameters.len()
        ));
    }

    // Generic positive feedback
    if review_points.is_empty() {
        review_points.push(format!(
            "The `{}` {} looks well-structured. The naming is clear and the logic \
             is straightforward.",
            symbol.name, symbol.kind
        ));
    }

    let assistant_1 = format!(
        "Here's my review of `{}`:\n\n{}",
        symbol.name,
        review_points
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{}. {p}", i + 1))
            .collect::<Vec<_>>()
            .join("\n\n")
    );

    // Turn 3: User asks for specific improvement
    let user_2 = if symbol.doc_comment.is_none() {
        "Can you show me what the documentation should look like?".to_string()
    } else if lines > 30 {
        "How would you refactor this to be shorter?".to_string()
    } else {
        "Any edge cases I should handle?".to_string()
    };

    // Turn 4: Detailed response
    let assistant_2 = if symbol.doc_comment.is_none() {
        generate_doc_suggestion(symbol, lang)
    } else if lines > 30 {
        format!(
            "Here's how I'd approach refactoring `{}`:\n\n\
             1. Identify the distinct responsibilities in the function\n\
             2. Extract each into a well-named helper function\n\
             3. The main function becomes an orchestrator that calls the helpers\n\n\
             This makes each piece independently testable and easier to understand.",
            symbol.name
        )
    } else {
        format!(
            "For `{}`, consider these edge cases:\n\n\
             1. What happens with empty or null inputs?\n\
             2. Are there boundary conditions for numeric parameters?\n\
             3. Could concurrent access cause issues?\n\n\
             Adding input validation and clear error handling for these cases \
             would make the code more robust.",
            symbol.name
        )
    };

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![file.clone()],
        sample_type: SampleType::MultiTurn,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_1),
            ConversationTurn::assistant(assistant_1),
            ConversationTurn::user(user_2),
            ConversationTurn::assistant(assistant_2),
        ],
        tools: None,
        metadata: SampleMetadata {
            language: Some(analysis.language.clone()),
            symbol_name: Some(symbol.name.clone()),
            symbol_kind: Some(symbol.kind.to_string()),
            estimated_tokens: symbol.source_text.len() / 4 + 400,
            complexity_tier: if lines > 30 {
                ComplexityTier::Complex
            } else {
                ComplexityTier::Moderate
            },
        },
    })
}

fn generate_doc_suggestion(symbol: &Symbol, lang: &crate::scanner::Language) -> String {
    use crate::scanner::Language;
    let name = &symbol.name;

    match lang {
        Language::Rust => {
            let params: String = symbol
                .parameters
                .iter()
                .filter(|p| p.name != "self" && p.name != "&self" && p.name != "&mut self")
                .map(|p| {
                    format!(
                        "/// * `{}` - TODO: describe this parameter",
                        p.name
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Here's a documentation template:\n\n```rust\n\
                 /// TODO: Describe what `{name}` does.\n\
                 ///\n\
                 /// # Arguments\n\
                 ///\n\
                 {params}\n\
                 ///\n\
                 /// # Returns\n\
                 ///\n\
                 /// TODO: Describe the return value.\n\
                 ```"
            )
        }
        Language::Python => {
            format!(
                "Here's a docstring template:\n\n```python\n\
                 def {name}(...):\n\
                     \"\"\"TODO: Describe what {name} does.\n\n\
                     Args:\n\
                         TODO: Document parameters\n\n\
                     Returns:\n\
                         TODO: Document return value\n\
                     \"\"\"\n\
                 ```"
            )
        }
        _ => {
            format!(
                "Here's a documentation template:\n\n```\n\
                 /**\n\
                  * TODO: Describe what {name} does.\n\
                  *\n\
                  * @param ... - TODO: Document parameters\n\
                  * @returns TODO: Document return value\n\
                  */\n\
                 ```"
            )
        }
    }
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
