mod error;
mod server;
mod spotify_client;
mod verification_util;

#[tokio::main]
async fn main() {
    let server = server::Server::new(true).await;
    server.run().await;
}
