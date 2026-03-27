pub mod alpaca;
pub mod chatml;
pub mod fim;
pub mod llama;
pub mod mistral;
pub mod openai;
pub mod sharegpt;

use crate::generator::types::TrainingSample;
use crate::pipeline::config::{OutputFormat, SampleType};

/// Format a training sample into a JSON value for the given output format.
pub fn format_sample(
    sample: &TrainingSample,
    format: &OutputFormat,
) -> serde_json::Value {
    // FIM samples get special handling
    if sample.sample_type == SampleType::CodeCompletion {
        return fim::format_fim(sample, format);
    }

    match format {
        OutputFormat::OpenAI => openai::format_openai(sample),
        OutputFormat::ChatML => chatml::format_chatml(sample),
        OutputFormat::Llama4 => llama::format_llama4(sample),
        OutputFormat::Mistral => mistral::format_mistral(sample),
        OutputFormat::ShareGPT => sharegpt::format_sharegpt(sample),
        OutputFormat::Alpaca => alpaca::format_alpaca(sample),
    }
}

/// Format a sample as a JSONL line (compact JSON string + newline).
pub fn format_as_jsonl_line(
    sample: &TrainingSample,
    format: &OutputFormat,
) -> String {
    let value = format_sample(sample, format);
    serde_json::to_string(&value).unwrap_or_default()
}
