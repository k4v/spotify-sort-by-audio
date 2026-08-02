use std::sync::Arc;

use axum::{Router, extract::Query, routing::get};
use tokio::{net::TcpListener, sync::Mutex};

use crate::{types::error, spotify_client::{SpotifyAuthCallbackParams, SpotifyClient}};

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
        Self {
            spotify_client: Arc::new(Mutex::new(SpotifyClient::new(&redirect_uri))),
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

    pub(crate) async fn run(&self) {
        self.spotify_client.lock().await.start_client_auth();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.server_config.server_port)).await.expect("Failed to bind to address");
        let router = self.get_configured_router();
        // Nothing to do with the result of axum::serve, as it will run indefinitely until the server is stopped
        axum::serve(listener, router).await.expect("Failed to start server");
    }

    fn get_configured_router(&self) -> Router {
        // Define required routes and REST handlers for the server
        let router: Router = Router::new()
            // Route to handle Spotify authorization callback after user login
            .route(Self::SPOTIFY_CALLBACK_PATH, get({
                let spotify_client = Arc::clone(&self.spotify_client);
                async move |auth_params: Query<SpotifyAuthCallbackParams>| {
                    if let Err(callback_err) = spotify_client.lock().await.handle_auth_callback(auth_params)
                        && matches!(callback_err, error::ServerError::TokenError(_)) {
                            spotify_client.lock().await.start_client_auth();
                        }
                }
            }))
            // Route to list playlists
            .route(Self::LIST_PLAYLISTS_PATH, get({
                let spotify_client = Arc::clone(&self.spotify_client);
                async move || {
                    match spotify_client.lock().await.list_playlists() {
                        Ok(playlists) => {
                            println!("Playlists: {:?}", playlists);
                        },
                        Err(list_playlists_err) => {
                            eprintln!("Error occurred while listing playlists: {}", list_playlists_err);
                        }
                    }
                }
            })
        );
        router
    }

}