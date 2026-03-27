use serde::{Deserialize, Serialize};

use crate::pipeline::config::SampleType;
use crate::scanner::Language;

/// A single training data sample ready for formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSample {
    pub id: String,
    pub source_project: String,
    pub source_files: Vec<String>,
    pub sample_type: SampleType,
    pub conversation: Vec<ConversationTurn>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub metadata: SampleMetadata,
}

/// A single turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationTurn {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ConversationTurn {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
    /// Llama 4 uses `ipython` for tool outputs
    IPython,
}

/// An OpenAI-style tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: serde_json::Value,
}

impl ToolCall {
    pub fn new(
        function_name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: format!("call_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0")),
            function_name: function_name.into(),
            arguments,
        }
    }
}

/// Metadata about a generated sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleMetadata {
    pub language: Option<Language>,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub estimated_tokens: usize,
    pub complexity_tier: ComplexityTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComplexityTier {
    Simple,
    Moderate,
    Complex,
}

/// Context passed to generators containing the analyzed codebase.
pub struct GenerationContext<'a> {
    pub project_name: &'a str,
    pub project_description: &'a str,
    pub analyses: &'a [crate::analyzer::CodeAnalysis],
    pub file_contents: &'a std::collections::HashMap<String, String>,
}
