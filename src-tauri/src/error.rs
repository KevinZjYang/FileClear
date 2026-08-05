use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Pdf(#[from] lopdf::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    #[error(transparent)]
    TempfilePersist(#[from] tempfile::PersistError),
    #[error(transparent)]
    Exif(#[from] kamadak_exif::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn msg(inner: impl Into<String>) -> Self {
        AppError::Message(inner.into())
    }
}
