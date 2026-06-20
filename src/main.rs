mod server;
mod spotify_client;
mod verification_util;

#[tokio::main]
async fn main() {
    let mut server = server::Server::new(true).await;
    server.run().await;

}
