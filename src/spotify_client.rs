#![allow(dead_code)]

use crate::verification_util;

pub(crate) struct SpotifyClient {
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    expiration_time: u64,
}

impl SpotifyClient {

    const AUTH_CALLBACK_PATH: &str = "/callback";
    const AUTH_LISTENER_PORT: u16 = 5000;
    const SPOTIFY_API_SCOPES: &str = "playlist-read-collaborative";

    pub(crate) fn new(load_env_file: bool) -> Self {

        let (client_id, client_secret) = load_config_from_env(load_env_file);

        Self {
            client_id,
            client_secret,
            access_token: None,
            expiration_time: 0,
        }
    }

    fn get_spotify_auth_url(&self) -> Result<http::Uri, http::Error> {
        let code_challenge = verification_util::build_code_challenge();

        let redirect_uri = format!(
            "http://127.0.0.1:{}{}",
            Self::AUTH_LISTENER_PORT,
            Self::AUTH_CALLBACK_PATH
        );

        let auth_url_path = format!(
            "/authorize?client_id={}&response_type=code&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={}",
            self.client_id,
            Self::SPOTIFY_API_SCOPES,
            redirect_uri,
            code_challenge
        );

        http::Uri::builder()
            .scheme("https")
            .authority("accounts.spotify.com")
            .path_and_query(auth_url_path)
            .build()
    }

    pub(crate) fn start_client_auth(&mut self) {

        let spotify_auth_url = self.get_spotify_auth_url().expect("Failed to build Spotify auth URL");
        println!("Authorize the application with Spotify by visiting the following URL: {}", spotify_auth_url);
    }

}

fn load_config_from_env(load_env_file: bool) -> (String, String) {
    if load_env_file {
        dotenv::dotenv().expect("Could you find .env file in current directory hierarchy");
    }

    let client_id = std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set");
    let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").expect("SPOTIFY_CLIENT_SECRET must be set");

    (client_id, client_secret)
}
