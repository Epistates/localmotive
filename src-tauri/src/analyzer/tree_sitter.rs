use std::collections::HashMap;
use std::sync::OnceLock;

use tree_sitter::Parser;
use tree_sitter_language::LanguageFn;

use crate::scanner::Language;

/// Registry mapping Language variants to tree-sitter LanguageFn.
/// Initialized once (OnceLock), read by all threads.
/// Parser instances are created on-demand since they are !Send.
pub struct ParserRegistry {
    languages: HashMap<Language, LanguageFn>,
}

impl ParserRegistry {
    fn new() -> Self {
        let mut languages = HashMap::new();
        languages.insert(Language::Rust, tree_sitter_rust::LANGUAGE);
        languages.insert(Language::JavaScript, tree_sitter_javascript::LANGUAGE);
        languages.insert(
            Language::TypeScript,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        );
        languages.insert(Language::Python, tree_sitter_python::LANGUAGE);
        languages.insert(Language::Go, tree_sitter_go::LANGUAGE);
        languages.insert(Language::Java, tree_sitter_java::LANGUAGE);
        languages.insert(Language::C, tree_sitter_c::LANGUAGE);
        languages.insert(Language::Cpp, tree_sitter_cpp::LANGUAGE);
        Self { languages }
    }

    /// Create a fresh Parser configured for the given language.
    /// Returns None if the language has no tree-sitter grammar.
    pub fn parser_for(&self, lang: &Language) -> Option<Parser> {
        let lang_fn = self.languages.get(lang)?;
        let mut parser = Parser::new();
        parser
            .set_language(&(*lang_fn).into())
            .expect("tree-sitter grammar version mismatch");
        Some(parser)
    }

    /// Check whether we have a grammar for a language.
    pub fn supports(&self, lang: &Language) -> bool {
        self.languages.contains_key(lang)
    }

    /// List all supported languages.
    pub fn supported_languages(&self) -> Vec<&Language> {
        self.languages.keys().collect()
    }
}

/// Global singleton registry.
static REGISTRY: OnceLock<ParserRegistry> = OnceLock::new();

pub fn registry() -> &'static ParserRegistry {
    REGISTRY.get_or_init(ParserRegistry::new)
}
