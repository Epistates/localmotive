use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::ast_extraction::{CodeAnalysis, Import};

/// Cross-file dependency information for a project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolTable {
    /// Map of file path → exported symbol names.
    pub exports: HashMap<String, Vec<String>>,
    /// Map of file path → imports from other files.
    pub dependencies: HashMap<String, Vec<Dependency>>,
    /// Total unique symbol count across the project.
    pub total_symbols: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub source_file: String,
    pub import_source: String,
    pub symbols: Vec<String>,
}

/// Build a cross-file symbol table from a collection of file analyses.
pub fn build_symbol_table(analyses: &[CodeAnalysis]) -> SymbolTable {
    let mut exports: HashMap<String, Vec<String>> = HashMap::new();
    let mut dependencies: HashMap<String, Vec<Dependency>> = HashMap::new();
    let mut total_symbols = 0usize;

    for analysis in analyses {
        let path = &analysis.file_path;

        // Collect exported symbols (public functions, classes, etc.)
        let file_exports: Vec<String> = analysis
            .symbols
            .iter()
            .map(|s| s.name.clone())
            .collect();
        total_symbols += file_exports.len();
        exports.insert(path.clone(), file_exports);

        // Collect dependencies from imports
        let file_deps: Vec<Dependency> = analysis
            .imports
            .iter()
            .map(|imp| import_to_dependency(imp, path))
            .collect();
        if !file_deps.is_empty() {
            dependencies.insert(path.clone(), file_deps);
        }
    }

    SymbolTable {
        exports,
        dependencies,
        total_symbols,
    }
}

fn import_to_dependency(import: &Import, file_path: &str) -> Dependency {
    Dependency {
        source_file: file_path.to_string(),
        import_source: import.source.clone(),
        symbols: import.symbols.clone(),
    }
}
