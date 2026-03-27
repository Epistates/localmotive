use serde_json::json;

use crate::generator::types::{Role, TrainingSample};

/// Format a sample as Alpaca format.
/// Simple instruction/input/output triplet.
/// Only suitable for single-turn Q&A samples; multi-turn is collapsed.
///
/// ```json
/// {"instruction": "...", "input": "...", "output": "..."}
/// ```
pub fn format_alpaca(sample: &TrainingSample) -> serde_json::Value {
    let mut instruction = String::new();
    let mut input = String::new();
    let mut output = String::new();

    for turn in &sample.conversation {
        match turn.role {
            Role::System => {
                instruction = turn.content.clone();
            }
            Role::User => {
                if input.is_empty() {
                    input = turn.content.clone();
                } else {
                    // Multi-turn: append subsequent user turns
                    input.push_str("\n\n");
                    input.push_str(&turn.content);
                }
            }
            Role::Assistant => {
                if output.is_empty() {
                    output = turn.content.clone();
                } else {
                    // Multi-turn: keep only the last assistant response
                    output = turn.content.clone();
                }
            }
            Role::Tool | Role::IPython => {
                // Append tool results as context to input
                input.push_str(&format!("\n\n[Tool result: {}]", turn.content));
            }
        }
    }

    // If no system message, use the user message as instruction
    if instruction.is_empty() {
        instruction = input.clone();
        input = String::new();
    }

    json!({
        "instruction": instruction,
        "input": input,
        "output": output,
    })
}
