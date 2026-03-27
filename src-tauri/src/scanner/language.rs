use std::path::Path;

use super::file_info::Language;

/// Detect the programming language of a file using hyperpolyglot.
/// Falls back to extension-based detection if hyperpolyglot can't determine it.
pub fn detect_language(path: &Path) -> Option<Language> {
    // Try hyperpolyglot first — it uses filename, extension, and content heuristics
    if let Ok(Some(detection)) = hyperpolyglot::detect(path) {
        return map_language_name(detection.language());
    }

    // Fallback: extension-based detection
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some(Language::Rust),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "py" | "pyw" | "pyi" => Some(Language::Python),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(Language::Cpp),
        "cs" => Some(Language::CSharp),
        "rb" => Some(Language::Ruby),
        "swift" => Some(Language::Swift),
        "kt" | "kts" => Some(Language::Kotlin),
        "php" => Some(Language::Php),
        "scala" | "sc" => Some(Language::Scala),
        "sh" | "bash" | "zsh" | "fish" => Some(Language::Shell),
        "lua" => Some(Language::Lua),
        "zig" => Some(Language::Zig),
        "ex" | "exs" => Some(Language::Elixir),
        "hs" | "lhs" => Some(Language::Haskell),
        "ml" | "mli" => Some(Language::Ocaml),
        "dart" => Some(Language::Dart),
        "html" | "htm" => Some(Language::Html),
        "css" | "scss" | "sass" | "less" => Some(Language::Css),
        "sql" => Some(Language::Sql),
        "md" | "markdown" => Some(Language::Markdown),
        "toml" => Some(Language::Toml),
        "yaml" | "yml" => Some(Language::Yaml),
        "json" | "jsonc" => Some(Language::Json),
        _ => None,
    }
}

/// Map hyperpolyglot language name to our Language enum.
fn map_language_name(name: &str) -> Option<Language> {
    match name {
        "Rust" => Some(Language::Rust),
        "TypeScript" | "TSX" => Some(Language::TypeScript),
        "JavaScript" | "JSX" => Some(Language::JavaScript),
        "Python" => Some(Language::Python),
        "Go" => Some(Language::Go),
        "Java" => Some(Language::Java),
        "C" => Some(Language::C),
        "C++" => Some(Language::Cpp),
        "C#" => Some(Language::CSharp),
        "Ruby" => Some(Language::Ruby),
        "Swift" => Some(Language::Swift),
        "Kotlin" => Some(Language::Kotlin),
        "PHP" => Some(Language::Php),
        "Scala" => Some(Language::Scala),
        "Shell" | "Bash" | "Zsh" | "Fish" => Some(Language::Shell),
        "Lua" => Some(Language::Lua),
        "Zig" => Some(Language::Zig),
        "Elixir" => Some(Language::Elixir),
        "Haskell" => Some(Language::Haskell),
        "OCaml" => Some(Language::Ocaml),
        "Dart" => Some(Language::Dart),
        "HTML" => Some(Language::Html),
        "CSS" | "SCSS" | "Sass" | "Less" => Some(Language::Css),
        "SQL" | "PLpgSQL" | "PLSQL" => Some(Language::Sql),
        "Markdown" => Some(Language::Markdown),
        "TOML" => Some(Language::Toml),
        "YAML" => Some(Language::Yaml),
        "JSON" => Some(Language::Json),
        other => Some(Language::Other(other.to_string())),
    }
}

/// Check whether a language has tree-sitter grammar support for AST analysis.
pub fn has_tree_sitter_support(lang: &Language) -> bool {
    matches!(
        lang,
        Language::Rust
            | Language::TypeScript
            | Language::JavaScript
            | Language::Python
            | Language::Go
            | Language::Java
            | Language::C
            | Language::Cpp
    )
}
