#![allow(dead_code)]

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
/// Error types that the Server would need to return, for example, related to
/// server configuration, authentication handling and Sopotify API interactions
pub(crate) enum ServerError {
    AuthError(String),
    ClientError(String),
    ConfigError(String),
    TokenError(String),
    InternalError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            ServerError::ClientError(msg) => write!(f, "Client error: {}", msg),
            ServerError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ServerError::TokenError(msg) => write!(f, "Token error: {}", msg),
            ServerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}