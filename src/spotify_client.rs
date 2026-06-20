#![allow(dead_code)]

use axum::extract::Query;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Deserialize)]
struct SpotifyAccessTokenResponseBody {
    access_token: String,
    token_type: String,
    scope: String,
    expires_in: String,
    refresh_token: String,
}

#[derive(Clone)]
struct CachedAccessToken {
    code_verifier: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expiration_time: u64,
}

#[derive(Clone)]
pub(crate) struct SpotifyClient {
    client_id: String,
    client_secret: String,
    access_token: CachedAccessToken,
    server_redirect_uri: String,
}

impl SpotifyClient {

    const SPOTIFY_AUTH_BASE_URL: &str = "accounts.spotify.com";
    const SPOTIFY_USER_AUTH_ENDPOINT: &str = "/authorize";
    const SPOTIFY_ACCESS_TOKEN_ENDPOINT: &str = "/api/token";
    const SPOTIFY_API_SCOPES: &str = "playlist-read-collaborative";

    pub(crate) fn new(server_redirect_uri: &str) -> Self {

        let (client_id, client_secret) = Self::load_config_from_env();

        Self {
            client_id,
            client_secret,
            access_token: CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expiration_time: 0 },
            server_redirect_uri: server_redirect_uri.to_string(),
        }
    }

    fn load_config_from_env() -> (String, String) {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set");
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").expect("SPOTIFY_CLIENT_SECRET must be set");

        (client_id, client_secret)
    }

    fn reset_spotify_access_token(&mut self) {
        self.access_token = CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expiration_time: 0 };
    }

    fn get_spotify_auth_url(&self) -> Result<(String, String), ()> {
        if let Ok((code_verifier, code_challenge)) = verification_util::build_code_challenge() {
            let auth_url_path = format!(
                "{}?client_id={}&response_type=code&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={}",
                Self::SPOTIFY_USER_AUTH_ENDPOINT,
                self.client_id,
                Self::SPOTIFY_API_SCOPES,
                self.server_redirect_uri,
                code_challenge
            );

            http::Uri::builder()
                .scheme("https")
                .authority(SpotifyClient::SPOTIFY_AUTH_BASE_URL)
                .path_and_query(auth_url_path)
                .build()
                .map(|uri| (uri.to_string(), code_verifier))
                .map_err(|_| ())

        } else {
            Err(())
        }
    }

    fn get_spotify_token_url(&self) -> Result<String, ()> {
        http::Uri::builder()
            .scheme("https")
            .authority(SpotifyClient::SPOTIFY_AUTH_BASE_URL)
            .path_and_query(Self::SPOTIFY_ACCESS_TOKEN_ENDPOINT)
            .build()
            .map(|uri| uri.to_string())
            .map_err(|_| ())
    }

    pub(crate) fn start_client_auth(&mut self) {
        // Reset existing access token container before starting new auth flow
        self.reset_spotify_access_token();

        let (spotify_auth_url, code_verifier) = self.get_spotify_auth_url().unwrap_or_else(|_| panic!("Failed to generate Spotify authorization URL"));
        self.access_token.code_verifier = Some(code_verifier);

        println!("Authorize the application with Spotify by visiting the following URL: {}", spotify_auth_url);
    }

    pub(crate) fn handle_auth_callback(&mut self, auth_params: Query<SpotifyAuthCallbackParams>) -> Result<(), ()> {
        // Ensure we have a code verifier cached, before comparing against client token
        // TODO: Abort callback flow instead?
        if self.access_token.code_verifier.is_none() {
            self.start_client_auth();
        }

        println!("Received Spotify auth callback with code: {} and state: {}", auth_params.code, auth_params.state.as_ref().unwrap_or(&"None".into()));
        if let Ok(token_url) = self.get_spotify_token_url() {
            println!("Spotify token endpoint URL: {}", token_url);
            let code_verifier = self.access_token.code_verifier.as_deref().unwrap_or("");
            let request_form = [
                ("grant_type", "authorization_code"),
                ("code", &auth_params.code),
                ("redirect_uri", &self.server_redirect_uri),
                ("client_id", &self.client_id),
                ("code_verifier", code_verifier),
            ];
            ureq::post(token_url)
                .content_type("application/x-www-form-urlencoded")
                .send_form(request_form)
                .map(|mut response| {
                    println!("Received response from Spotify token endpoint: {:?}", response);
                    if let Ok(access_token) = response.body_mut().read_json::<SpotifyAccessTokenResponseBody>() {
                        self.access_token.access_token = Some(access_token.access_token);
                        self.access_token.refresh_token = Some(access_token.refresh_token);
                    }
                })
                .map_err(|_| ())
        } else {
            Err(())
        }
    }

}
