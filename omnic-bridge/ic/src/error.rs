// bridge error

use derive_more::{Display, From};
use serde_json::Error as SerdeError;
use crate::pool::Error as PoolError;
use crate::token::Error as TokenError;

pub type Result<T = ()> = std::result::Result<T, Error>;

/// Errors which can occur when attempting to generate resource uri.
#[derive(Debug, Display, From)]
pub enum Error {
    /// decoder error
    #[display(fmt = "Decode payload error: {}", _0)]
    Decoder(String),
    /// invalid cmd
    #[display(fmt = "Got invalid command: {}", _0)]
    #[from(ignore)]
    InvalidOpetion(String),
    /// token error
    #[display(fmt = "Token error: {}", _0)]
    #[from(ignore)]
    Token(TokenError),
    /// pool error
    #[display(fmt = "Pool error: {}", _0)]
    #[from(ignore)]
    Pool(PoolError),
    /// internal error
    #[display(fmt = "Internal error")]
    Internal,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::Error::*;
        match *self {
            Decoder(_) | InvalidOpetion(_) | Token { .. }| Pool { .. } | Internal => None,
        }
    }
}

impl From<SerdeError> for Error {
    fn from(err: SerdeError) -> Self {
        Error::Decoder(format!("{:?}", err))
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        use self::Error::*;
        match self {
            Decoder(s) => Decoder(s.clone()),
            InvalidOpetion(s) => InvalidOpetion(s.clone()),
            Token(e) => Token(e.clone()),
            Pool(e) => Pool(e.clone()),
            Internal => Internal,
        }
    }
}
