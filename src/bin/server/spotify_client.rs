#![allow(dead_code)]

use std::{sync::{Arc, Mutex, mpsc::{self, RecvTimeoutError}}, thread::{self, JoinHandle}, time::{Duration, SystemTime}};

use axum::extract::Query;
use serde::Deserialize;
use url::Url;

use crate::verification_util;

struct ClientConfig {
    client_id: String,
    client_secret: Option<String>,  // Only required for server-to-server requests, not needed for PKCE flow
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyAuthCallbackParams {
    code: String,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyAccessTokenResponseBody {
    access_token: String,
    token_type: String,
    scope: String,
    expires_in: u64,
    refresh_token: String,
}

struct CachedAccessToken {
    code_verifier: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: SystemTime,
}

pub(crate) struct SpotifyClient {
    client_id: String,
    cached_access_token: Arc<Mutex<CachedAccessToken>>,
    server_redirect_uri: String,
    shutdown_tx: mpsc::Sender<()>,
    refresh_thread: Option<JoinHandle<()>>,
}

impl CachedAccessToken {
    fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }
}

impl SpotifyClient {

    const SPOTIFY_AUTH_BASE_URL: &str = "accounts.spotify.com";
    const SPOTIFY_USER_AUTH_ENDPOINT: &str = "/authorize";
    const SPOTIFY_ACCESS_TOKEN_ENDPOINT: &str = "/api/token";
    const SPOTIFY_API_SCOPES: &str = "playlist-read-private playlist-read-collaborative";

    pub(crate) fn new(server_redirect_uri: &str) -> Self {

        let ClientConfig { client_id, client_secret: _ } = Self::load_config_from_env();
        let cached_access_token = Arc::new(Mutex::new(CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expires_at: SystemTime::UNIX_EPOCH }));
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

        let refresh_thread = Self::build_refresh_thread(cached_access_token.clone(), client_id.clone(), shutdown_rx);

        Self {
            client_id,
            cached_access_token,
            server_redirect_uri: server_redirect_uri.to_string(),
            shutdown_tx,
            refresh_thread: Some(refresh_thread),
        }
    }

    fn build_refresh_thread(access_token: Arc<Mutex<CachedAccessToken>>, client_id: String, shutdown_rx: mpsc::Receiver<()>) -> JoinHandle<()> {

        thread::spawn(move || loop {
            match shutdown_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    let needs_refresh = {
                        let token = access_token.lock().unwrap();
                        token.is_expired()
                    };

                    if needs_refresh {
                        // Clone the code_verifier out of the mutex so it lives long enough
                        let mut cached_token_guard = access_token.lock().unwrap();
                        if cached_token_guard.refresh_token.is_none() {
                            println!("No refresh token available, cannot refresh Spotify access token");
                            continue;
                        }
                        let request_form = [
                            ("grant_type", "refresh_token"),
                            ("refresh_token", cached_token_guard.refresh_token.as_ref().unwrap()),  // Safe to unwrap since we already checked for empty earlier
                            ("client_id", &client_id),
                        ];
                        let token_refresh_url = Self::get_spotify_token_url();
                        if token_refresh_url.is_err() {
                            println!("Error building Spotify token refresh URL: {}", token_refresh_url.err().unwrap());
                            continue;
                        }
                        let _ = ureq::post(token_refresh_url.unwrap())
                            .send_form(request_form)
                            .map(|mut response| {
                                if let Ok(access_token) = response.body_mut().read_json::<SpotifyAccessTokenResponseBody>() {
                                    // Ensure the code verifier is the one that was sent to the Token generation URL
                                    cached_token_guard.access_token = Some(access_token.access_token.clone());
                                    cached_token_guard.refresh_token = Some(access_token.refresh_token.clone());
                                    cached_token_guard.expires_at = SystemTime::now() + Duration::from_secs(access_token.expires_in - 60);
                                    println!("Received a Spotify access token expiring in {} seconds", access_token.expires_in);
                                } else {
                                    cached_token_guard.access_token = None;
                                    cached_token_guard.refresh_token = None;
                                }
                            })
                            .map_err(|error| format!("Error processing access token response: {}", error));
                    }
                }
            }
        })
    }

    fn load_config_from_env() -> ClientConfig {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set");
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").ok();

        ClientConfig { client_id, client_secret }
    }

    fn reset_spotify_access_token(&mut self) {
        let mut guard = self.cached_access_token.lock().unwrap();
        *guard = CachedAccessToken { code_verifier: None, access_token: None, refresh_token: None, expires_at: SystemTime::UNIX_EPOCH };
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

    fn get_spotify_token_url() -> Result<String, String> {
        Url::parse(&format!("https://{}{}", Self::SPOTIFY_AUTH_BASE_URL, Self::SPOTIFY_ACCESS_TOKEN_ENDPOINT))
            .map(|access_token_url| access_token_url.to_string())
            .map_err(|error| format!("Error building access token URL: {}", error))
    }

    pub(crate) fn start_client_auth(&mut self) {
        // Reset existing access token container before starting new auth flow
        self.reset_spotify_access_token();

        let (spotify_auth_url, code_verifier) = self.get_spotify_auth_url().unwrap_or_else(|_| panic!("Failed to generate Spotify authorization URL"));
        self.cached_access_token.lock().unwrap().code_verifier = Some(code_verifier);

        println!("Authorize with Spotify: {}", spotify_auth_url);
    }

    pub(crate) fn handle_auth_callback(&mut self, auth_params: Query<SpotifyAuthCallbackParams>) -> Result<(), String> {
        match Self::get_spotify_token_url() {
            Ok(token_url) => {
                // Clone the code_verifier out of the mutex so it lives long enough
                let mut cached_token_guard = self.cached_access_token.lock().unwrap();
                let code_verifier = cached_token_guard.code_verifier.clone().ok_or("Code verifier not found in cached access token".to_string())?.clone();
                let request_form = [
                    ("grant_type", "authorization_code"),
                    ("code", &auth_params.code),
                    ("redirect_uri", &self.server_redirect_uri),
                    ("client_id", &self.client_id),
                    ("code_verifier", &code_verifier),
                ];
                ureq::post(token_url)
                    .send_form(request_form)
                    .map(|mut response| {
                        if let Ok(access_token) = response.body_mut().read_json::<SpotifyAccessTokenResponseBody>() {
                            // Ensure the code verifier is the one that was sent to the Token generation URL
                            cached_token_guard.code_verifier.replace(code_verifier);
                            cached_token_guard.access_token = Some(access_token.access_token.clone());
                            cached_token_guard.refresh_token = Some(access_token.refresh_token.clone());
                            cached_token_guard.expires_at = SystemTime::now() + Duration::from_secs(access_token.expires_in - 60);
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
