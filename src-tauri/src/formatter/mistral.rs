use serde_json::json;

use crate::generator::types::{Role, TrainingSample};

/// Format a sample as Mistral format.
/// Uses [INST]/[/INST] control tokens with JSON tool_calls.
/// Mistral requires `id` fields on all tool calls.
/// System message is prepended to the first user [INST] block.
pub fn format_mistral(sample: &TrainingSample) -> serde_json::Value {
    let mut text = String::new();
    text.push_str("<s>");

    // Collect system content to prepend to first user turn
    let system_content: Option<String> = sample.conversation.iter().find_map(|t| {
        if t.role == Role::System {
            let mut sys = t.content.clone();
            if let Some(tools) = &sample.tools {
                sys.push_str("\n\nAvailable tools:\n");
                for tool in tools {
                    let def = json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters,
                        }
                    });
                    sys.push_str(&serde_json::to_string(&def).unwrap_or_default());
                    sys.push('\n');
                }
            }
            Some(sys)
        } else {
            None
        }
    });

    let mut system_prepended = false;

    for turn in &sample.conversation {
        match turn.role {
            Role::System => {
                // Handled above — system content prepended to first user turn
                continue;
            }
            Role::User => {
                text.push_str("[INST] ");
                // Prepend system content to first user turn
                if !system_prepended {
                    if let Some(sys) = &system_content {
                        text.push_str(sys);
                        text.push_str("\n\n");
                    }
                    system_prepended = true;
                }
                text.push_str(&turn.content);
                text.push_str(" [/INST]");
            }
            Role::Assistant => {
                text.push_str(&turn.content);

                // Tool calls with required `id` (Mistral-specific)
                if let Some(calls) = &turn.tool_calls {
                    text.push_str("\n[TOOL_CALLS] ");
                    let formatted: Vec<serde_json::Value> = calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "name": c.function_name,
                                "arguments": c.arguments,
                            })
                        })
                        .collect();
                    text.push_str(
                        &serde_json::to_string(&formatted).unwrap_or_default(),
                    );
                }

                text.push_str("</s>");
            }
            Role::Tool | Role::IPython => {
                let id = turn.tool_call_id.as_deref().unwrap_or("unknown");
                text.push_str(&format!(
                    "[TOOL_RESULTS] {{\"id\": \"{}\", \"content\": {}}}[/TOOL_RESULTS]",
                    id,
                    serde_json::to_string(&turn.content).unwrap_or_default()
                ));
            }
        }
    }

    json!({ "text": text })
}
