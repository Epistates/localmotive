use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Parse error in {file}: {message}")]
    ParseError { file: String, message: String },

    #[error("Tree-sitter error: {0}")]
    TreeSitter(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Generation cancelled")]
    Cancelled,

    #[error("Export error: {0}")]
    Export(String),

    #[error("Scanner error: {0}")]
    Scanner(String),

    #[error("Path error: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
