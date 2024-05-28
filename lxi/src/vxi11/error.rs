use super::sunrpc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Other(String),

    #[error("io error occurred: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("{0}")]
    DecodeError(String),

    #[error("{source}")]
    FromIntError {
        #[from]
        source: std::num::TryFromIntError,
    },

    #[error("{source}")]
    RpcError {
        #[from]
        source: sunrpc::error::Error
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

