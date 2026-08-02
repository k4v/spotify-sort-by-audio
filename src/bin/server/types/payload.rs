#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyAccessTokenResponseBody {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
    pub expires_in: u64,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyExternalUrls {
    pub spotify: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyImage {
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyUser {
    pub display_name: Option<String>,
    pub external_urls: SpotifyExternalUrls,
    pub href: String,
    pub id: String,
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyTracks {
    pub href: String,
    pub total: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyPlaylistItem {
    pub collaborative: bool,
    pub description: Option<String>,
    pub external_urls: SpotifyExternalUrls,
    pub href: String,
    pub id: String,
    pub images: Vec<SpotifyImage>,
    pub name: String,
    pub owner: SpotifyUser,
    pub public: Option<bool>,
    pub snapshot_id: String,
    pub items: Option<Vec<SpotifyTracks>>,
    pub tracks: SpotifyTracks,
    pub r#type: String,
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpotifyListPlaylistsResponse {
    pub href: String,
    pub limit: u32,
    pub next: Option<String>,
    pub offset: u32,
    pub previous: Option<String>,
    pub total: u32,
    pub items: Vec<SpotifyPlaylistItem>,
}
