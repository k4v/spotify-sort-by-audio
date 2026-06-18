mod spotify_client;
mod verification_util;

fn main() {
    let mut spotify_client = spotify_client::SpotifyClient::new(true);
    spotify_client.start_client_auth();
}
