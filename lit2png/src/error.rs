#[derive(Debug, thiserror::Error)]
pub enum LitError {
    #[error("bad magic: not a LIT file")]
    BadMagic,
    #[error("bad dimensions {0}x{1}")]
    BadDims(u32, u32),
    #[error("file truncated: need {0} bytes, have {1}")]
    Truncated(usize, usize),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = LitError> = std::result::Result<T, E>;
