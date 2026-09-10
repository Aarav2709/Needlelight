use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Failed to parse XML: {0}")]
    Xml(#[from] quick_xml::de::DeError),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Hash mismatch - the downloaded file may be corrupted.")]
    HashMismatch,
    #[error("Custom ModLinks configuration is invalid.")]
    InvalidModlinks,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidInput(String),
}

pub type AppResult<T> = Result<T, AppError>;
