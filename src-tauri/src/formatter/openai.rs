use serde_json::json;

use crate::generator::types::{ConversationTurn, Role, TrainingSample};

/// Format a sample as OpenAI Chat format (the universal standard).
/// Used by GPT-OSS, Axolotl, and as the general interchange format.
///
/// ```json
/// {"messages": [{"role": "system", "content": "..."}, ...], "tools": [...]}
/// ```
pub fn format_openai(sample: &TrainingSample) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = sample
        .conversation
        .iter()
        .map(|turn| format_turn(turn))
        .collect();

    let mut obj = json!({ "messages": messages });

    if let Some(tools) = &sample.tools {
        let tool_defs: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        obj["tools"] = json!(tool_defs);
    }

    obj
}

fn format_turn(turn: &ConversationTurn) -> serde_json::Value {
    let role = match turn.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool | Role::IPython => "tool",
    };

    let mut msg = json!({
        "role": role,
        "content": turn.content,
    });

    if let Some(calls) = &turn.tool_calls {
        let formatted_calls: Vec<serde_json::Value> = calls
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "type": "function",
                    "function": {
                        "name": c.function_name,
                        "arguments": c.arguments.to_string(),
                    }
                })
            })
            .collect();
        msg["tool_calls"] = json!(formatted_calls);
    }

    if let Some(id) = &turn.tool_call_id {
        msg["tool_call_id"] = json!(id);
    }

    if let Some(name) = &turn.name {
        msg["name"] = json!(name);
    }

    msg
}
