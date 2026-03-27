use crate::analyzer::CodeAnalysis;
use crate::pipeline::config::SampleType;

use super::types::*;
use super::Generator;

/// Generates cross-file reasoning training data.
/// Creates samples that require understanding file relationships and project structure.
pub struct RepoContextGenerator;

impl Generator for RepoContextGenerator {
    fn sample_type(&self) -> SampleType {
        SampleType::RepoContext
    }

    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample> {
        let mut samples = Vec::new();

        // Only generate if we have multiple files to reason across
        if ctx.analyses.len() < 2 {
            return samples;
        }

        // Pattern 1: Project overview
        if let Some(sample) = generate_project_overview(ctx) {
            samples.push(sample);
        }

        // Pattern 2: Module interaction (for files that import each other)
        for (i, analysis_a) in ctx.analyses.iter().enumerate() {
            for analysis_b in ctx.analyses.iter().skip(i + 1) {
                if files_are_related(analysis_a, analysis_b) {
                    if let Some(sample) =
                        generate_module_interaction(ctx, analysis_a, analysis_b)
                    {
                        samples.push(sample);
                    }
                }
            }
        }

        samples
    }
}

/// Check if two files are related (one imports the other or shares symbols).
fn files_are_related(a: &CodeAnalysis, b: &CodeAnalysis) -> bool {
    // Check if a imports from b or vice versa
    let a_imports_b = a
        .imports
        .iter()
        .any(|imp| imp.source.contains(&file_stem(&b.file_path)));
    let b_imports_a = b
        .imports
        .iter()
        .any(|imp| imp.source.contains(&file_stem(&a.file_path)));

    // Check if they share symbol references
    let shared_symbols = a.symbols.iter().any(|sa| {
        b.symbols
            .iter()
            .any(|sb| sa.source_text.contains(&sb.name) || sb.source_text.contains(&sa.name))
    });

    a_imports_b || b_imports_a || shared_symbols
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Generate a "describe this project" training sample.
fn generate_project_overview(ctx: &GenerationContext) -> Option<TrainingSample> {
    let file_count = ctx.analyses.len();
    let total_symbols: usize = ctx.analyses.iter().map(|a| a.symbols.len()).sum();

    if total_symbols < 3 {
        return None;
    }

    let system_prompt = format!(
        "You are a senior developer. Provide clear, structured overviews of codebases."
    );

    let user_msg = format!(
        "Give me an overview of the {} project.",
        ctx.project_name
    );

    // Build overview from analysis data
    let mut overview = String::new();

    if !ctx.project_description.is_empty() {
        overview.push_str(&format!("**{}** — {}\n\n", ctx.project_name, ctx.project_description));
    } else {
        overview.push_str(&format!("**{}**\n\n", ctx.project_name));
    }

    overview.push_str(&format!(
        "The project contains {} source file(s) with {} symbol(s).\n\n",
        file_count, total_symbols
    ));

    overview.push_str("**Key files:**\n");
    for analysis in ctx.analyses.iter().take(8) {
        let symbols: Vec<&str> = analysis
            .symbols
            .iter()
            .take(3)
            .map(|s| s.name.as_str())
            .collect();
        if !symbols.is_empty() {
            overview.push_str(&format!(
                "- `{}` ({}) — defines: {}\n",
                analysis.file_path,
                analysis.language,
                symbols.join(", ")
            ));
        }
    }

    let all_files: Vec<String> = ctx.analyses.iter().map(|a| a.file_path.clone()).collect();

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: all_files,
        sample_type: SampleType::RepoContext,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_msg),
            ConversationTurn::assistant(overview),
        ],
        tools: None,
        metadata: SampleMetadata {
            language: None,
            symbol_name: None,
            symbol_kind: None,
            estimated_tokens: 300,
            complexity_tier: ComplexityTier::Moderate,
        },
    })
}

/// Generate a "how do modules A and B interact?" training sample.
fn generate_module_interaction(
    ctx: &GenerationContext,
    a: &CodeAnalysis,
    b: &CodeAnalysis,
) -> Option<TrainingSample> {
    let system_prompt = format!(
        "You are a senior developer. Explain code architecture and module interactions clearly."
    );

    let user_msg = format!(
        "How do `{}` and `{}` interact in the {} project?",
        a.file_path, b.file_path, ctx.project_name
    );

    // Find shared symbols
    let a_symbols_in_b: Vec<&str> = a
        .symbols
        .iter()
        .filter(|s| b.symbols.iter().any(|sb| sb.source_text.contains(&s.name)))
        .map(|s| s.name.as_str())
        .take(5)
        .collect();

    let b_symbols_in_a: Vec<&str> = b
        .symbols
        .iter()
        .filter(|s| a.symbols.iter().any(|sa| sa.source_text.contains(&s.name)))
        .map(|s| s.name.as_str())
        .take(5)
        .collect();

    let mut answer = format!(
        "`{}` and `{}` are related modules in the project.\n\n",
        a.file_path, b.file_path
    );

    if !a_symbols_in_b.is_empty() {
        answer.push_str(&format!(
            "`{}` defines {} which is referenced in `{}`: {}\n\n",
            a.file_path,
            if a_symbols_in_b.len() == 1 { "a symbol" } else { "symbols" },
            b.file_path,
            a_symbols_in_b.join(", ")
        ));
    }

    if !b_symbols_in_a.is_empty() {
        answer.push_str(&format!(
            "`{}` defines {} referenced in `{}`: {}\n\n",
            b.file_path,
            if b_symbols_in_a.len() == 1 { "a symbol" } else { "symbols" },
            a.file_path,
            b_symbols_in_a.join(", ")
        ));
    }

    if a_symbols_in_b.is_empty() && b_symbols_in_a.is_empty() {
        answer.push_str(
            "These files share related functionality but don't directly reference \
             each other's exports.",
        );
    }

    Some(TrainingSample {
        id: uuid::Uuid::new_v4().to_string(),
        source_project: ctx.project_name.to_string(),
        source_files: vec![a.file_path.clone(), b.file_path.clone()],
        sample_type: SampleType::RepoContext,
        conversation: vec![
            ConversationTurn::system(system_prompt),
            ConversationTurn::user(user_msg),
            ConversationTurn::assistant(answer),
        ],
        tools: None,
        metadata: SampleMetadata {
            language: Some(a.language.clone()),
            symbol_name: None,
            symbol_kind: None,
            estimated_tokens: 350,
            complexity_tier: ComplexityTier::Complex,
        },
    })
}
