use rand::prelude::*;
use sha2::{Digest, Sha256};
use base64::{Engine, prelude::BASE64_STANDARD};

// Build a code verifier for client authorization flow with PKCE
// and generate a code challenge from the code verifier value
pub(crate) fn build_code_challenge() -> String {
    const CODE_VERIFIER_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    const CODE_VERIFIER_LENGTH: usize = 128;

    let mut rng = rand::rng();
    // Generate a random string of length 128 from the allowed character set
    let code_verifier: Vec<u8> = (0..CODE_VERIFIER_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CODE_VERIFIER_CHARSET.len());
            CODE_VERIFIER_CHARSET[idx] as u8
        })
        .collect();

    // Create a SHA256 hash of the code verifier
    let hash = Sha256::digest(code_verifier);

    // Encode the hash in base64url format
    let code_challenge = BASE64_STANDARD.encode(&hash);
    code_challenge
}
