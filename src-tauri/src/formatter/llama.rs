use serde_json::json;

use crate::generator::types::{Role, TrainingSample};

/// Format a sample as Llama 4 format.
/// Uses start_header_id/end_header_id tokens and pythonic tool calling.
///
/// ```text
/// <|begin_of_text|><|start_header_id|>system<|end_header_id|>
///
/// You are helpful<|eot_id|>
/// <|start_header_id|>user<|end_header_id|>
///
/// Hello<|eot_id|>
/// <|start_header_id|>assistant<|end_header_id|>
///
/// Hi<|eot_id|>
/// ```
///
/// Tool calls use Python-like syntax: `[func(arg=val)]`
/// Tool results use the `ipython` role.
pub fn format_llama4(sample: &TrainingSample) -> serde_json::Value {
    let mut text = String::new();
    text.push_str("<|begin_of_text|>");

    for turn in &sample.conversation {
        let role = match turn.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool | Role::IPython => "ipython",
        };

        text.push_str(&format!("<|start_header_id|>{role}<|end_header_id|>\n\n"));

        // System message with tool definitions
        if turn.role == Role::System && sample.tools.is_some() {
            text.push_str(&turn.content);
            text.push_str("\n\nYou have access to the following tools:\n\n");
            if let Some(tools) = &sample.tools {
                for tool in tools {
                    let def = serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                    });
                    text.push_str(&serde_json::to_string(&def).unwrap_or_default());
                    text.push('\n');
                }
            }
        } else {
            text.push_str(&turn.content);
        }

        // Tool calls as pythonic expressions
        if let Some(calls) = &turn.tool_calls {
            text.push('\n');
            let call_strs: Vec<String> = calls
                .iter()
                .map(|c| {
                    let args = format_pythonic_args(&c.arguments);
                    format!("{}({})", c.function_name, args)
                })
                .collect();
            text.push_str(&format!("[{}]", call_strs.join(", ")));
        }

        text.push_str("<|eot_id|>\n");
    }

    json!({ "text": text })
}

/// Format JSON arguments as Python-style keyword arguments.
/// `{"path": "foo.rs", "start_line": 1}` → `path='foo.rs', start_line=1`
fn format_pythonic_args(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let val = match v {
                        serde_json::Value::String(s) => format!("'{s}'"),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => {
                            if *b { "True" } else { "False" }.to_string()
                        }
                        serde_json::Value::Null => "None".to_string(),
                        other => other.to_string(),
                    };
                    format!("{k}={val}")
                })
                .collect();
            parts.join(", ")
        }
        _ => args.to_string(),
    }
}
