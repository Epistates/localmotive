use serde::{Deserialize, Serialize};
use tree_sitter::Node;

use crate::scanner::Language;

/// A code symbol extracted from AST analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub source_text: String,
    pub doc_comment: Option<String>,
    pub signature: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub parent: Option<String>,
    pub visibility: Visibility,
    pub body_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    Constant,
    Module,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Method => write!(f, "method"),
            Self::Class => write!(f, "class"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Interface => write!(f, "interface"),
            Self::Trait => write!(f, "trait"),
            Self::Type => write!(f, "type"),
            Self::Constant => write!(f, "constant"),
            Self::Module => write!(f, "module"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub start_line: usize,
    pub end_line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Default,
}

/// An import statement extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    pub source: String,
    pub symbols: Vec<String>,
    pub is_default: bool,
    pub is_wildcard: bool,
}

/// Complete analysis of a single source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeAnalysis {
    pub file_path: String,
    pub language: Language,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
}

/// Extract symbols and imports from source code using tree-sitter.
pub fn extract_symbols(source: &str, tree: &tree_sitter::Tree, language: &Language) -> CodeAnalysis {
    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut imports = Vec::new();

    match language {
        Language::Rust => extract_rust_symbols(root, source, &mut symbols, &mut imports, None),
        Language::TypeScript | Language::JavaScript => {
            extract_ts_js_symbols(root, source, &mut symbols, &mut imports, None)
        }
        Language::Python => extract_python_symbols(root, source, &mut symbols, &mut imports, None),
        Language::Go => extract_go_symbols(root, source, &mut symbols, &mut imports, None),
        Language::Java => extract_java_symbols(root, source, &mut symbols, &mut imports, None),
        Language::C | Language::Cpp => {
            extract_c_cpp_symbols(root, source, &mut symbols, &mut imports, None)
        }
        _ => {}
    }

    CodeAnalysis {
        file_path: String::new(), // Set by caller
        language: language.clone(),
        symbols,
        imports,
    }
}

// ── Rust ───────────────────────────────────────────────────────────────

fn extract_rust_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(sym) = parse_rust_function(child, source, parent_name) {
                    symbols.push(sym);
                }
            }
            "struct_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let doc = preceding_doc_comment(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Struct,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: doc,
                        signature: Some(format!("struct {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "enum_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let doc = preceding_doc_comment(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Enum,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: doc,
                        signature: Some(format!("enum {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "trait_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let doc = preceding_doc_comment(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Trait,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: doc,
                        signature: Some(format!("trait {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "impl_item" => {
                // Extract the type being implemented
                let impl_type = child_by_field(child, "type", source)
                    .or_else(|| child_by_field(child, "name", source));
                let parent = impl_type.as_deref().or(parent_name);
                // Recurse into impl body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    extract_rust_symbols(body, source, symbols, imports, parent);
                }
            }
            "use_declaration" => {
                if let Some(imp) = parse_rust_use(child, source) {
                    imports.push(imp);
                }
            }
            "mod_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Module,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(format!("mod {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: None,
                    });
                }
            }
            "type_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Type,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: None,
                    });
                }
            }
            "const_item" | "static_item" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Constant,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: rust_visibility(child, source),
                        body_text: None,
                    });
                }
            }
            _ => {
                // Recurse into other containers
                extract_rust_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

fn parse_rust_function(node: Node, source: &str, parent_name: Option<&str>) -> Option<Symbol> {
    let name = child_by_field(node, "name", source)?;
    let doc = preceding_doc_comment(node, source);
    let params = parse_rust_parameters(node, source);
    let return_type = child_by_field(node, "return_type", source);

    let kind = if parent_name.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };

    // Build signature: fn name(params) -> ReturnType
    let params_str = params
        .iter()
        .map(|p| {
            if let Some(ty) = &p.type_annotation {
                format!("{}: {ty}", p.name)
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sig = if let Some(ret) = &return_type {
        format!("fn {name}({params_str}) -> {ret}")
    } else {
        format!("fn {name}({params_str})")
    };

    let body_text = node
        .child_by_field_name("body")
        .map(|b| node_text(b, source));

    Some(Symbol {
        name,
        kind,
        span: node_span(node),
        source_text: node_text(node, source),
        doc_comment: doc,
        signature: Some(sig),
        parameters: params,
        return_type,
        parent: parent_name.map(String::from),
        visibility: rust_visibility(node, source),
        body_text,
    })
}

fn parse_rust_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let Some(params_node) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "parameter" | "self_parameter" => {
                let name = child_by_field(child, "pattern", source)
                    .or_else(|| Some(node_text(child, source)));
                let type_ann = child_by_field(child, "type", source);
                if let Some(name) = name {
                    params.push(Parameter {
                        name,
                        type_annotation: type_ann,
                        is_optional: false,
                    });
                }
            }
            _ => {}
        }
    }
    params
}

fn parse_rust_use(node: Node, source: &str) -> Option<Import> {
    let text = node_text(node, source);
    // Simple heuristic parse of use statements
    let trimmed = text.trim_start_matches("pub ").trim_start_matches("use ").trim_end_matches(';');
    let is_wildcard = trimmed.ends_with("::*");
    Some(Import {
        source: trimmed.to_string(),
        symbols: Vec::new(),
        is_default: false,
        is_wildcard,
    })
}

fn rust_visibility(node: Node, source: &str) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = node_text(child, source);
            if text.contains("pub") {
                return Visibility::Public;
            }
        }
    }
    Visibility::Private
}

// ── TypeScript / JavaScript ────────────────────────────────────────────

fn extract_ts_js_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if let Some(sym) = parse_ts_function(child, source, parent_name) {
                    symbols.push(sym);
                }
            }
            "class_declaration" => {
                let name = child_by_field(child, "name", source);
                if let Some(name) = &name {
                    let doc = preceding_doc_comment(child, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Class,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: doc,
                        signature: Some(format!("class {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
                // Recurse into class body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    extract_ts_js_symbols(body, source, symbols, imports, name.as_deref());
                }
            }
            "interface_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Interface,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(format!("interface {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "type_alias_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Type,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: None,
                    });
                }
            }
            "enum_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Enum,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(format!("enum {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "method_definition" => {
                if let Some(sym) = parse_ts_method(child, source, parent_name) {
                    symbols.push(sym);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                // Handle: export const foo = ...  or const foo = () => {}
                extract_ts_variable_symbols(child, source, symbols, parent_name);
            }
            "export_statement" => {
                // Recurse into exports to find the actual declaration
                extract_ts_js_symbols(child, source, symbols, imports, parent_name);
            }
            "import_statement" => {
                if let Some(imp) = parse_ts_import(child, source) {
                    imports.push(imp);
                }
            }
            _ => {
                extract_ts_js_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

fn parse_ts_function(node: Node, source: &str, parent_name: Option<&str>) -> Option<Symbol> {
    let name = child_by_field(node, "name", source)?;
    let doc = preceding_doc_comment(node, source);
    let params = parse_ts_parameters(node, source);
    let return_type = child_by_field(node, "return_type", source)
        .map(|r| r.trim_start_matches(':').trim().to_string());

    let params_str = params
        .iter()
        .map(|p| {
            let opt = if p.is_optional { "?" } else { "" };
            if let Some(ty) = &p.type_annotation {
                format!("{}{opt}: {ty}", p.name)
            } else {
                format!("{}{opt}", p.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let sig = if let Some(ret) = &return_type {
        format!("function {name}({params_str}): {ret}")
    } else {
        format!("function {name}({params_str})")
    };

    Some(Symbol {
        name,
        kind: SymbolKind::Function,
        span: node_span(node),
        source_text: node_text(node, source),
        doc_comment: doc,
        signature: Some(sig),
        parameters: params,
        return_type,
        parent: parent_name.map(String::from),
        visibility: Visibility::Default,
        body_text: node.child_by_field_name("body").map(|b| node_text(b, source)),
    })
}

fn parse_ts_method(node: Node, source: &str, parent_name: Option<&str>) -> Option<Symbol> {
    let name = child_by_field(node, "name", source)?;
    let doc = preceding_doc_comment(node, source);
    let params = parse_ts_parameters(node, source);
    let return_type = child_by_field(node, "return_type", source)
        .map(|r| r.trim_start_matches(':').trim().to_string());

    Some(Symbol {
        name: name.clone(),
        kind: SymbolKind::Method,
        span: node_span(node),
        source_text: node_text(node, source),
        doc_comment: doc,
        signature: Some(format!("{}()", name)),
        parameters: params,
        return_type,
        parent: parent_name.map(String::from),
        visibility: Visibility::Default,
        body_text: node.child_by_field_name("body").map(|b| node_text(b, source)),
    })
}

fn parse_ts_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let Some(params_node) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "required_parameter"
            || child.kind() == "optional_parameter"
            || child.kind() == "formal_parameters"
        {
            let name = child_by_field(child, "pattern", source)
                .or_else(|| child_by_field(child, "name", source))
                .unwrap_or_else(|| node_text(child, source));
            let type_ann = child_by_field(child, "type", source);
            let is_optional = child.kind() == "optional_parameter";
            params.push(Parameter {
                name,
                type_annotation: type_ann,
                is_optional,
            });
        }
    }
    params
}

fn extract_ts_variable_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name = child_by_field(child, "name", source);
            let value = child.child_by_field_name("value");
            if let (Some(name), Some(value)) = (name, value) {
                // Check if value is an arrow function or function expression
                if value.kind() == "arrow_function" || value.kind() == "function" {
                    let params = parse_ts_parameters(value, source);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        span: node_span(node),
                        source_text: node_text(node, source),
                        doc_comment: preceding_doc_comment(node, source),
                        signature: Some(format!("const {name} = ...")),
                        parameters: params,
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: value.child_by_field_name("body").map(|b| node_text(b, source)),
                    });
                }
            }
        }
    }
}

fn parse_ts_import(node: Node, source: &str) -> Option<Import> {
    let text = node_text(node, source);
    let source_module = child_by_field(node, "source", source)
        .map(|s| s.trim_matches(|c: char| c == '\'' || c == '"').to_string())
        .unwrap_or_default();
    Some(Import {
        source: source_module,
        symbols: Vec::new(), // Could parse import specifiers, kept simple for now
        is_default: text.contains("import ") && !text.contains('{'),
        is_wildcard: text.contains("* as"),
    })
}

// ── Python ─────────────────────────────────────────────────────────────

fn extract_python_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                let name = child_by_field(child, "name", source);
                if let Some(name) = name {
                    let kind = if parent_name.is_some() {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: python_docstring(child, source),
                        signature: Some(extract_first_line(child, source)),
                        parameters: parse_python_parameters(child, source),
                        return_type: child_by_field(child, "return_type", source),
                        parent: parent_name.map(String::from),
                        visibility: if name.starts_with('_') {
                            Visibility::Private
                        } else {
                            Visibility::Public
                        },
                        body_text: child.child_by_field_name("body").map(|b| node_text(b, source)),
                    });
                }
            }
            "class_definition" => {
                let name = child_by_field(child, "name", source);
                if let Some(name) = &name {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Class,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: python_docstring(child, source),
                        signature: Some(format!("class {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Public,
                        body_text: Some(node_text(child, source)),
                    });
                }
                if let Some(body) = child.child_by_field_name("body") {
                    extract_python_symbols(body, source, symbols, imports, name.as_deref());
                }
            }
            "import_statement" | "import_from_statement" => {
                imports.push(Import {
                    source: node_text(child, source),
                    symbols: Vec::new(),
                    is_default: false,
                    is_wildcard: node_text(child, source).contains('*'),
                });
            }
            _ => {
                extract_python_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

fn parse_python_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let Some(params_node) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "typed_parameter" || child.kind() == "default_parameter" {
            let name = child_by_field(child, "name", source)
                .unwrap_or_else(|| node_text(child, source));
            if name == "self" || name == "cls" {
                continue;
            }
            let type_ann = child_by_field(child, "type", source);
            params.push(Parameter {
                name,
                type_annotation: type_ann,
                is_optional: child.kind() == "default_parameter",
            });
        }
    }
    params
}

fn python_docstring(node: Node, source: &str) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "string" {
                    let text = node_text(inner, source);
                    let trimmed = text
                        .trim_start_matches("\"\"\"")
                        .trim_end_matches("\"\"\"")
                        .trim_start_matches("'''")
                        .trim_end_matches("'''")
                        .trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
        break; // Only check the first statement
    }
    None
}

// ── Go ─────────────────────────────────────────────────────────────────

fn extract_go_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(extract_first_line(child, source)),
                        parameters: Vec::new(),
                        return_type: child_by_field(child, "result", source),
                        parent: parent_name.map(String::from),
                        visibility: vis,
                        body_text: child.child_by_field_name("body").map(|b| node_text(b, source)),
                    });
                }
            }
            "method_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Method,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(extract_first_line(child, source)),
                        parameters: Vec::new(),
                        return_type: child_by_field(child, "result", source),
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: child.child_by_field_name("body").map(|b| node_text(b, source)),
                    });
                }
            }
            "type_declaration" => {
                extract_go_symbols(child, source, symbols, imports, parent_name);
            }
            "type_spec" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let kind = if node_text(child, source).contains("interface") {
                        SymbolKind::Interface
                    } else {
                        SymbolKind::Struct
                    };
                    symbols.push(Symbol {
                        name,
                        kind,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "import_declaration" => {
                imports.push(Import {
                    source: node_text(child, source),
                    symbols: Vec::new(),
                    is_default: false,
                    is_wildcard: false,
                });
            }
            _ => {
                extract_go_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

// ── Java ───────────────────────────────────────────────────────────────

fn extract_java_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                let name = child_by_field(child, "name", source);
                if let Some(name) = &name {
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Class,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(format!("class {name}")),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Public,
                        body_text: Some(node_text(child, source)),
                    });
                }
                if let Some(body) = child.child_by_field_name("body") {
                    extract_java_symbols(body, source, symbols, imports, name.as_deref());
                }
            }
            "interface_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Interface,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Public,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "method_declaration" | "constructor_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Method,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(extract_first_line(child, source)),
                        parameters: Vec::new(),
                        return_type: child_by_field(child, "type", source),
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Public,
                        body_text: child.child_by_field_name("body").map(|b| node_text(b, source)),
                    });
                }
            }
            "enum_declaration" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Enum,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Public,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "import_declaration" => {
                imports.push(Import {
                    source: node_text(child, source),
                    symbols: Vec::new(),
                    is_default: false,
                    is_wildcard: node_text(child, source).contains('*'),
                });
            }
            _ => {
                extract_java_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

// ── C / C++ ────────────────────────────────────────────────────────────

fn extract_c_cpp_symbols(
    node: Node,
    source: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut Vec<Import>,
    parent_name: Option<&str>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                let declarator = child.child_by_field_name("declarator");
                if let Some(decl) = declarator {
                    let name = find_identifier_in_declarator(decl, source);
                    if let Some(name) = name {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Function,
                            span: node_span(child),
                            source_text: node_text(child, source),
                            doc_comment: preceding_doc_comment(child, source),
                            signature: Some(extract_first_line(child, source)),
                            parameters: Vec::new(),
                            return_type: child_by_field(child, "type", source),
                            parent: parent_name.map(String::from),
                            visibility: Visibility::Default,
                            body_text: child
                                .child_by_field_name("body")
                                .map(|b| node_text(b, source)),
                        });
                    }
                }
            }
            "struct_specifier" | "class_specifier" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    let kind = if child.kind() == "class_specifier" {
                        SymbolKind::Class
                    } else {
                        SymbolKind::Struct
                    };
                    symbols.push(Symbol {
                        name,
                        kind,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(extract_first_line(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "enum_specifier" => {
                if let Some(name) = child_by_field(child, "name", source) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Enum,
                        span: node_span(child),
                        source_text: node_text(child, source),
                        doc_comment: preceding_doc_comment(child, source),
                        signature: Some(node_text(child, source)),
                        parameters: Vec::new(),
                        return_type: None,
                        parent: parent_name.map(String::from),
                        visibility: Visibility::Default,
                        body_text: Some(node_text(child, source)),
                    });
                }
            }
            "preproc_include" => {
                imports.push(Import {
                    source: node_text(child, source),
                    symbols: Vec::new(),
                    is_default: false,
                    is_wildcard: false,
                });
            }
            _ => {
                extract_c_cpp_symbols(child, source, symbols, imports, parent_name);
            }
        }
    }
}

fn find_identifier_in_declarator(node: Node, source: &str) -> Option<String> {
    if node.kind() == "identifier" || node.kind() == "field_identifier" {
        return Some(node_text(node, source));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(name) = find_identifier_in_declarator(child, source) {
            return Some(name);
        }
    }
    None
}

// ── Common helpers ─────────────────────────────────────────────────────

fn node_text(node: Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

fn node_span(node: Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row + 1,
        end_line: end.row + 1,
        start_col: start.column,
        end_col: end.column,
        byte_start: node.start_byte(),
        byte_end: node.end_byte(),
    }
}

fn child_by_field(node: Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| node_text(n, source))
}

fn extract_first_line(node: Node, source: &str) -> String {
    let text = node_text(node, source);
    text.lines().next().unwrap_or("").to_string()
}

/// Extract doc comment immediately preceding a node.
/// Handles `///` (Rust), `/** */` (JS/Java), and `#` (Python) style comments.
fn preceding_doc_comment(node: Node, source: &str) -> Option<String> {
    let mut prev = node.prev_sibling();
    let mut comments = Vec::new();

    while let Some(sibling) = prev {
        let kind = sibling.kind();
        if kind == "line_comment" || kind == "comment" || kind == "block_comment" {
            let text = node_text(sibling, source);
            comments.push(text);
            prev = sibling.prev_sibling();
        } else {
            break;
        }
    }

    if comments.is_empty() {
        return None;
    }

    comments.reverse();
    let combined = comments
        .iter()
        .map(|c| {
            c.trim()
                .trim_start_matches("///")
                .trim_start_matches("//!")
                .trim_start_matches("//")
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .trim_start_matches("/*")
                .trim_start_matches('*')
                .trim_start_matches('#')
                .trim()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}
