#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("rejected: RPC mismatch low = {low}, high = {high}")]
    RpcMismatch { low: u32, high: u32 },

    #[error("rejected: bad credentials")]
    BadCredentials,

    #[error("rejected: rejected credentials")]
    RejectedCredentials,

    #[error("rejected: bad verifier")]
    BadVerifier,

    #[error("rejected: rejected verifier")]
    RejectedVerifier,

    #[error("rejected: authentication too weak")]
    AuthenticationTooWeak,

    #[error("program unavailable")]
    ProgramUnavailable,

    #[error("program mismatch low = {low}, high = {high}")]
    ProgramMismatch { low: u32, high: u32 },

    #[error("procedure unavailable")]
    ProcedureUnavailable,

    #[error("garbage arguments")]
    GarbageArgs,

    #[error("unable to be decode RPC message")]
    DecodeFailed,

    #[error("unable to encode RPC message")]
    EncodeFailed,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, self::Error>;

