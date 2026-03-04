use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Serde error")]
    Serde(#[from] serde_json::Error),
}