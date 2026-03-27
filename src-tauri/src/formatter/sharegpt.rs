use serde_json::json;

use crate::generator::types::{Role, TrainingSample};

/// Format a sample as ShareGPT format.
/// Legacy format still consumed by LlamaFactory and Unsloth.
///
/// ```json
/// {"conversations": [{"from": "human", "value": "..."}, {"from": "gpt", "value": "..."}]}
/// ```
///
/// Note: ShareGPT does not support tool calling. Tool calls are flattened
/// into assistant text in conversational mode.
pub fn format_sharegpt(sample: &TrainingSample) -> serde_json::Value {
    let mut conversations = Vec::new();

    for turn in &sample.conversation {
        let from = match turn.role {
            Role::System => "system",
            Role::User => "human",
            Role::Assistant => "gpt",
            Role::Tool | Role::IPython => "observation",
        };

        let mut value = turn.content.clone();

        // Flatten tool calls into text for ShareGPT
        if let Some(calls) = &turn.tool_calls {
            for call in calls {
                value.push_str(&format!(
                    "\n\n[Called {}: {}]",
                    call.function_name,
                    call.arguments
                ));
            }
        }

        conversations.push(json!({
            "from": from,
            "value": value,
        }));
    }

    json!({ "conversations": conversations })
}
