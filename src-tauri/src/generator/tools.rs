use serde_json::json;

use super::types::ToolDefinition;

/// Canonical set of code assistant tools for training data.
/// These teach models to interact with codebases via structured tool use.
pub fn code_assistant_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file at the given path.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path relative to the project root."
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Optional starting line number (1-indexed)."
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Optional ending line number (inclusive)."
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "search_code".to_string(),
            description: "Search for a pattern across files in the project. Returns matching lines with file paths and line numbers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The search pattern (supports regex)."
                    },
                    "file_glob": {
                        "type": "string",
                        "description": "Optional glob to filter files (e.g., '*.rs', 'src/**/*.ts')."
                    }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List files and directories at the given path.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path relative to the project root."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively. Default false."
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Make an edit to a file by replacing old text with new text.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path relative to the project root."
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find and replace."
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The replacement text."
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        ToolDefinition {
            name: "run_command".to_string(),
            description: "Execute a shell command in the project directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "find_references".to_string(),
            description: "Find all references to a symbol (function, class, variable) across the project.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name to search for."
                    },
                    "file_glob": {
                        "type": "string",
                        "description": "Optional glob to narrow the search scope."
                    }
                },
                "required": ["symbol"]
            }),
        },
        ToolDefinition {
            name: "get_symbol_info".to_string(),
            description: "Get detailed information about a code symbol including its type, parameters, documentation, and location.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name."
                    },
                    "file": {
                        "type": "string",
                        "description": "Optional file path to narrow the lookup."
                    }
                },
                "required": ["symbol"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or completely overwrite an existing file.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path relative to the project root."
                    },
                    "content": {
                        "type": "string",
                        "description": "The complete file content to write."
                    }
                },
                "required": ["path", "content"]
            }),
        },
    ]
}

/// Get a subset of tools appropriate for read-only operations.
pub fn readonly_tools() -> Vec<ToolDefinition> {
    code_assistant_tools()
        .into_iter()
        .filter(|t| {
            matches!(
                t.name.as_str(),
                "read_file"
                    | "search_code"
                    | "list_directory"
                    | "find_references"
                    | "get_symbol_info"
            )
        })
        .collect()
}
