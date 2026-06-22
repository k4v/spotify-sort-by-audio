#![allow(dead_code)]

use std::time::{Duration, SystemTime};

use axum::extract::Query;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::verification_util;

#[derive(Deserialize)]
pub(crate) struct SpotifyAuthCallbackParams {
    code: String,
    state: Option<String>,
}

#[derive(Clone, Serialize)]
struct SpotifyAccessTokenRequestBody {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    code_verifier: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SpotifyAccessTokenResponseBody {
    access_token: String,
    token_type: String,
    scope: String,
    expires_in: u64,
    refresh_token: String,
}

#[derive(Clone)]
struct CachedAccessToken {
    code_verifier: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: SystemTime,
}

#[derive(Clone)]
pub(crate) struct SpotifyClient {
    client_id: String,
    client_secret: String,
    cached_access_token: CachedAccessToken,
    server_redirect_uri: String,
}

impl SpotifyClient {

    const SPOTIFY_AUTH_BASE_URL: &str = "accounts.spotify.com";
    const SPOTIFY_USER_AUTH_ENDPOINT: &str = "/authorize";
    const SPOTIFY_ACCESS_TOKEN_ENDPOINT: &str = "/api/token";
    const SPOTIFY_API_SCOPES: &str = "playlist-read-private playlist-read-collaborative";

    pub(crate) fn new(server_redirect_uri: &str) -> Self {

        let (client_id, client_secret) = Self::load_config_from_env();

        Self {
            client_id,
            client_secret,
            cached_access_token: CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expires_at: SystemTime::UNIX_EPOCH },
            server_redirect_uri: server_redirect_uri.to_string(),
        }
    }

    fn load_config_from_env() -> (String, String) {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set");
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").expect("SPOTIFY_CLIENT_SECRET must be set");

        (client_id, client_secret)
    }

    fn reset_spotify_access_token(&mut self) {
        self.cached_access_token = CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expires_at: SystemTime::UNIX_EPOCH };
    }

    fn get_spotify_auth_url(&self) -> Result<(String, String), String> {
        if let Ok((code_verifier, code_challenge)) = verification_util::build_code_challenge() {
            Url::parse(&format!("https://{}{}", Self::SPOTIFY_AUTH_BASE_URL, Self::SPOTIFY_USER_AUTH_ENDPOINT))
                .map(|mut auth_url| {
                    auth_url.query_pairs_mut()
                        .append_pair("client_id", &self.client_id)
                        .append_pair("response_type", "code")
                        .append_pair("redirect_uri", &self.server_redirect_uri)
                        .append_pair("scope", Self::SPOTIFY_API_SCOPES)
                        .append_pair("code_challenge_method", "S256")
                        .append_pair("code_challenge", &code_challenge);
                    (auth_url.to_string(), code_verifier)
                })
                .map_err(|_| "Unable to build authorization URL".to_string())
        } else {
            Err("Error building code challenge".to_string())
        }
    }

    fn get_spotify_token_url(&self) -> Result<String, String> {
        Url::parse(&format!("https://{}{}", Self::SPOTIFY_AUTH_BASE_URL, Self::SPOTIFY_ACCESS_TOKEN_ENDPOINT))
            .map(|access_token_url| access_token_url.to_string())
            .map_err(|error| format!("Error building access token URL: {}", error))
    }

    pub(crate) fn start_client_auth(&mut self) {
        // Reset existing access token container before starting new auth flow
        self.reset_spotify_access_token();

        let (spotify_auth_url, code_verifier) = self.get_spotify_auth_url().unwrap_or_else(|_| panic!("Failed to generate Spotify authorization URL"));
        self.cached_access_token.code_verifier = Some(code_verifier);

        println!("Authorize with Spotify: {}", spotify_auth_url);
    }

    pub(crate) fn handle_auth_callback(&mut self, auth_params: Query<SpotifyAuthCallbackParams>) -> Result<(), String> {
        // Ensure we have a code verifier cached, before comparing against client token
        // TODO (*): Abort callback flow instead?
        if self.cached_access_token.code_verifier.is_none() {
            self.start_client_auth();
        }

        match self.get_spotify_token_url() {
            Ok(token_url) => {
                let code_verifier = self.cached_access_token.code_verifier.as_deref().unwrap_or("");
                let request_form = [
                    ("grant_type", "authorization_code"),
                    ("code", &auth_params.code),
                    ("redirect_uri", &self.server_redirect_uri),
                    ("client_id", &self.client_id),
                    ("code_verifier", code_verifier),
                ];
                ureq::post(token_url)
                    .send_form(request_form)
                    .map(|mut response| {
                        if let Ok(access_token) = response.body_mut().read_json::<SpotifyAccessTokenResponseBody>() {
                            self.cached_access_token.access_token = Some(access_token.access_token);
                            self.cached_access_token.refresh_token = Some(access_token.refresh_token);
                            // TODO (***): Implement background refresh thread
                            self.cached_access_token.expires_at = SystemTime::now() + Duration::from_secs(access_token.expires_in - 60);
                            println!("Received a Spotify access token expiring in {} seconds", access_token.expires_in);
                        }
                    })
                    .map_err(|error| format!("Error processing access token response: {}", error))
            },
            Err(error) => {
                Err(error)
            }
        }
    }

}
