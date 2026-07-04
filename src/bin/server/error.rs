#![allow(dead_code)]

#[allow(clippy::enum_variant_names)]
pub(crate) enum ServerError {
    ClientError(String),
    AuthError(String),
    TokenError(String),
}