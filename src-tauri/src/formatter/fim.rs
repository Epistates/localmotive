use serde_json::json;

use crate::generator::types::{Role, TrainingSample};
use crate::pipeline::config::OutputFormat;

/// Format a code completion sample in FIM (Fill-in-the-Middle) format.
/// Different models use different special tokens for FIM.
///
/// The conversation is expected to have:
/// - System: context
/// - User: contains `<FILL>` marker
/// - Assistant: the fill content
pub fn format_fim(sample: &TrainingSample, target_format: &OutputFormat) -> serde_json::Value {
    let user_content = sample
        .conversation
        .iter()
        .find(|t| t.role == Role::User)
        .map(|t| &t.content)
        .unwrap_or(&String::new())
        .clone();

    let assistant_content = sample
        .conversation
        .iter()
        .find(|t| t.role == Role::Assistant)
        .map(|t| &t.content)
        .unwrap_or(&String::new())
        .clone();

    // Extract code from markdown code blocks
    let code_with_fill = extract_code_block(&user_content).unwrap_or(&user_content);
    let fill_content = extract_code_block(&assistant_content).unwrap_or(&assistant_content);

    // Split on the <FILL> marker
    let parts: Vec<&str> = code_with_fill.splitn(2, "<FILL>").collect();
    if parts.len() != 2 {
        // Fallback: return as regular chat format
        return json!({
            "prefix": code_with_fill,
            "middle": fill_content,
            "suffix": "",
        });
    }

    let prefix = parts[0];
    let suffix = parts[1];

    // Apply model-specific FIM tokens
    match target_format {
        OutputFormat::OpenAI => {
            // OpenAI FIM uses their specific tokens
            json!({
                "prompt": format!("<|fim_prefix|>{prefix}<|fim_suffix|>{suffix}<|fim_middle|>"),
                "completion": fill_content,
            })
        }
        OutputFormat::ChatML => {
            // Qwen/StarCoder style FIM
            json!({
                "text": format!(
                    "<fim_prefix>{prefix}<fim_suffix>{suffix}<fim_middle>{fill_content}",
                )
            })
        }
        OutputFormat::Llama4 => {
            // Llama Code FIM
            json!({
                "text": format!(
                    "<PRE> {prefix} <SUF>{suffix} <MID> {fill_content}",
                )
            })
        }
        _ => {
            // Generic FIM format (prefix/middle/suffix)
            json!({
                "prefix": prefix,
                "middle": fill_content,
                "suffix": suffix,
            })
        }
    }
}

/// Extract code from a markdown code block.
fn extract_code_block(text: &str) -> Option<&str> {
    let start = text.find("```")? + 3;
    // Skip the language identifier on the first line
    let code_start = text[start..].find('\n')? + start + 1;
    let end = text[code_start..].find("```")? + code_start;
    Some(&text[code_start..end])
}
