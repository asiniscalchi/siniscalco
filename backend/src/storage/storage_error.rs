#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Validation(&'static str),
    #[error("{0}")]
    Internal(&'static str),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}
