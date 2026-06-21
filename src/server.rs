#![allow(dead_code)]
use std::sync::Arc;

use axum::{Router, extract::Query, routing::get};
use tokio::{net::TcpListener, sync::Mutex};

use crate::spotify_client::{SpotifyAuthCallbackParams, SpotifyClient};

struct ServerConfig {
    server_port: u16,
}

pub(crate) struct Server {
    // Spotify authorization client to manage access tokens
    spotify_client: Arc<Mutex<SpotifyClient>>,
    server_config: ServerConfig,
}
impl Server {
    // Environment variable keys for server configurations
    const SERVER_CONFIG_PORT_KEY: &str = "SERVER_PORT";

    // Available REST endpoint routes for the server
    const SPOTIFY_CALLBACK_PATH: &str = "/callback";
    const LIST_PLAYLISTS_PATH: &str = "/playlists";

    pub(crate) async fn new(load_env_file_config: bool) -> Self {
        let server_config = Self::load_config_from_env(load_env_file_config);

        let redirect_uri = format!("http://127.0.0.1:{}{}", server_config.server_port, Self::SPOTIFY_CALLBACK_PATH);
        let spotify_client = SpotifyClient::new(&redirect_uri);

        Self {
            spotify_client: Arc::new(Mutex::new(spotify_client)),
            server_config
        }
    }

    fn load_config_from_env(load_env_file: bool) -> ServerConfig {
        if load_env_file {
            dotenv::dotenv().expect("Could you find .env file in current directory hierarchy");
        }

        let server_port_str = std::env::var(Self::SERVER_CONFIG_PORT_KEY).unwrap_or_else(|_| panic!("{} must be set in environment variables", Self::SERVER_CONFIG_PORT_KEY));
        let server_port = server_port_str.parse::<u16>().unwrap_or_else(|_| panic!("{} must be a valid u16", Self::SERVER_CONFIG_PORT_KEY));
        ServerConfig { server_port }
    }

    pub(crate) async fn run(&mut self) {
        self.spotify_client.lock().await.start_client_auth();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.server_config.server_port)).await.expect("Failed to bind to address");
        let router = self.get_configured_router();
        axum::serve(listener, router).await.expect("Failed to start server");
    }

    fn get_configured_router(&mut self) -> Router {
        // Define required routes and REST handlers for the server
        let router: Router = Router::new()
            // Route to handle Spotify authorization callback after user login
            .route(Self::SPOTIFY_CALLBACK_PATH, get({
                let spotify_client = Arc::clone(&self.spotify_client);
                async move |auth_params: Query<SpotifyAuthCallbackParams>| {
                    if let Err(callback_err) = spotify_client.lock().await.handle_auth_callback(auth_params) {
                        println!("Error handling user access callback: {}", callback_err);
                    }
                }
            }))
            // Route to list playlists
            .route(Self::LIST_PLAYLISTS_PATH, get(Self::list_playlists)
        );
        router
    }

    async fn list_playlists() -> String {
        "List of playlists".to_string()
    }

}