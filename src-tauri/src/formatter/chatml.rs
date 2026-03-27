use serde_json::json;

use crate::generator::types::{Role, TrainingSample};

/// Format a sample as ChatML / Hermes format.
/// Used by Qwen3, Qwen3.5, Phi-4.
///
/// ```text
/// <|im_start|>system
/// You are helpful<|im_end|>
/// <|im_start|>user
/// Hello<|im_end|>
/// <|im_start|>assistant
/// Hi<|im_end|>
/// ```
///
/// Tool definitions go in `<tools></tools>` XML tags in the system message.
/// Tool calls use `<tool_call></tool_call>` XML tags.
pub fn format_chatml(sample: &TrainingSample) -> serde_json::Value {
    let mut text = String::new();

    for turn in &sample.conversation {
        let role = match turn.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool | Role::IPython => "tool",
        };

        text.push_str(&format!("<|im_start|>{role}\n"));

        // Inject tool definitions into system message
        if turn.role == Role::System {
            text.push_str(&turn.content);
            if let Some(tools) = &sample.tools {
                text.push_str("\n\n<tools>\n");
                for tool in tools {
                    let def = serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters,
                        }
                    });
                    text.push_str(&serde_json::to_string(&def).unwrap_or_default());
                    text.push('\n');
                }
                text.push_str("</tools>");
            }
        } else {
            text.push_str(&turn.content);
        }

        // Tool calls in assistant messages
        if let Some(calls) = &turn.tool_calls {
            for call in calls {
                text.push_str("\n<tool_call>\n");
                let call_json = json!({
                    "name": call.function_name,
                    "arguments": call.arguments,
                });
                text.push_str(&serde_json::to_string_pretty(&call_json).unwrap_or_default());
                text.push_str("\n</tool_call>");
            }
        }

        text.push_str("<|im_end|>\n");
    }

    json!({ "text": text })
}
