use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("xml parse error: {0}")]
    Xml(#[from] quick_xml::de::DeError),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("hash mismatch")]
    HashMismatch,
    #[error("invalid custom modlinks")]
    InvalidModlinks,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type AppResult<T> = Result<T, AppError>;
