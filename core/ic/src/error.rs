

use ic_web3::ethabi;

/// Error types for Nomad
#[derive(Debug, thiserror::Error)]
pub enum OmnicError {
    /// IO error from Read/Write usage
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ABIDecodeError(#[from] ethabi::Error),

    #[error("decode failed: `{0}`")]
    DecodeError(String),

    #[error("db error: `{0}`")]
    DBError(String),

    #[error("home error: `{0}`")]
    ClientError(String),

    #[error(transparent)]
    RPCError(#[from] ic_web3::error::Error),

    #[error(transparent)]
    ProveError(#[from] accumulator::error::ProvingError),

    #[error("other: `{0}`")]
    Other(String),
}