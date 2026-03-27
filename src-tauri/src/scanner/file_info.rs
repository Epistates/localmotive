use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Detected programming language of a source file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Swift,
    Kotlin,
    Php,
    Scala,
    Shell,
    Lua,
    Zig,
    Elixir,
    Haskell,
    Ocaml,
    Dart,
    Html,
    Css,
    Sql,
    Markdown,
    Toml,
    Yaml,
    Json,
    Other(String),
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::Python => write!(f, "Python"),
            Self::Go => write!(f, "Go"),
            Self::Java => write!(f, "Java"),
            Self::C => write!(f, "C"),
            Self::Cpp => write!(f, "C++"),
            Self::CSharp => write!(f, "C#"),
            Self::Ruby => write!(f, "Ruby"),
            Self::Swift => write!(f, "Swift"),
            Self::Kotlin => write!(f, "Kotlin"),
            Self::Php => write!(f, "PHP"),
            Self::Scala => write!(f, "Scala"),
            Self::Shell => write!(f, "Shell"),
            Self::Lua => write!(f, "Lua"),
            Self::Zig => write!(f, "Zig"),
            Self::Elixir => write!(f, "Elixir"),
            Self::Haskell => write!(f, "Haskell"),
            Self::Ocaml => write!(f, "OCaml"),
            Self::Dart => write!(f, "Dart"),
            Self::Html => write!(f, "HTML"),
            Self::Css => write!(f, "CSS"),
            Self::Sql => write!(f, "SQL"),
            Self::Markdown => write!(f, "Markdown"),
            Self::Toml => write!(f, "TOML"),
            Self::Yaml => write!(f, "YAML"),
            Self::Json => write!(f, "JSON"),
            Self::Other(name) => write!(f, "{name}"),
        }
    }
}

/// Metadata about a single source file in a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: String,
    pub language: Option<Language>,
    pub size_bytes: u64,
    pub line_count: usize,
    pub is_binary: bool,
    /// xxh3 hash of file content, used for dedup and caching.
    pub content_hash: u64,
}
