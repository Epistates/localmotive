pub mod bug_detection;
pub mod code_completion;
pub mod code_qa;
pub mod documentation;
pub mod multi_turn;
pub mod repo_context;
pub mod tool_calling;
pub mod tools;
pub mod types;

use crate::pipeline::config::SampleType;
pub use types::*;

/// Generator trait — the MCP-ready seam.
///
/// All current implementations are template-based (deterministic, no LLM).
/// Future hybrid implementations can swap in MCP-backed generators via
/// this same trait without touching the pipeline.
pub trait Generator: Send + Sync {
    /// Generate training samples from the provided context.
    fn generate(&self, ctx: &GenerationContext) -> Vec<TrainingSample>;

    /// The sample type this generator produces.
    fn sample_type(&self) -> SampleType;
}

/// Run all enabled generators against a context and collect samples.
pub fn generate_all(
    ctx: &GenerationContext,
    enabled_types: &[SampleType],
) -> Vec<TrainingSample> {
    let generators: Vec<Box<dyn Generator>> = vec![
        Box::new(code_qa::CodeQAGenerator),
        Box::new(documentation::DocumentationGenerator),
        Box::new(code_completion::CodeCompletionGenerator),
        Box::new(tool_calling::ToolCallingGenerator),
        Box::new(multi_turn::MultiTurnGenerator),
        Box::new(bug_detection::BugDetectionGenerator),
        Box::new(repo_context::RepoContextGenerator),
    ];

    let mut samples = Vec::new();
    for gen in &generators {
        if enabled_types.contains(&gen.sample_type()) {
            samples.extend(gen.generate(ctx));
        }
    }
    samples
}
