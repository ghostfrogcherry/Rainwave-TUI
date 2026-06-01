use anyhow::{Result, anyhow};
use image::imageops::FilterType;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::models::*;

const API_BASE: &str = "https://rainwave.cc/api4";

#[derive(Clone)]
pub struct RainwaveApi {
    pub client: Client,
    pub user_id: Option<i32>,
    pub api_key: Option<String>,
    pub station_id: i32,
}

impl RainwaveApi {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            user_id: None,
            api_key: None,
            station_id: 1,
        }
    }

    pub fn with_auth(user_id: i32, api_key: String, station_id: i32) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            user_id: Some(user_id),
            api_key: Some(api_key),
            station_id,
        }
    }

    fn get_params(&self) -> Vec<(&str, String)> {
        let mut params = vec![("sid", self.station_id.to_string())];
        if let Some(uid) = self.user_id {
            if let Some(key) = &self.api_key {
                params.push(("user_id", uid.to_string()));
                params.push(("key", key.clone()));
            }
        }
        params
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<UserInfo> {
        let url = format!("{}/oauth_login", API_BASE);
        let params = [("username", username), ("password", password)];

        #[derive(Deserialize)]
        struct LoginResult {
            success: bool,
            code: Option<i32>,
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct LoginResponse {
            login_result: LoginResult,
            user_id: Option<i32>,
            api_key: Option<String>,
            username: Option<String>,
            listener_points: Option<i32>,
        }

        let resp = self.client
            .post(&url)
            .form(&params)
            .send()
            .await?
            .json::<LoginResponse>()
            .await?;

        if resp.login_result.success {
            Ok(UserInfo {
                user_id: resp.user_id.unwrap_or(0),
                username: resp.username.unwrap_or_default(),
                api_key: resp.api_key.unwrap_or_default(),
                listener_points: resp.listener_points.unwrap_or(0),
                avatar: None,
            })
        } else {
            Err(anyhow!(resp.login_result.text.unwrap_or_else(|| "Login failed".to_string())))
        }
    }

    pub async fn get_stations(&self) -> Result<Vec<Station>> {
        let url = format!("{}/stations", API_BASE);

        #[derive(Deserialize)]
        struct StationsResponse {
            stations: Vec<Station>,
        }

        let resp = self.client
            .get(&url)
            .send()
            .await?
            .json::<StationsResponse>()
            .await?;

        Ok(resp.stations)
    }

    pub async fn get_info(&self) -> Result<SyncData> {
        let url = format!("{}/info", API_BASE);
        let params = self.get_params();

        let resp_text = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?
            .text()
            .await?;

        let mut data = serde_json::from_str::<SyncData>(&resp_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        if let Some(current) = data.sched_current.as_mut() {
            for song in &mut current.songs {
                normalize_song(song);
            }
            if current.song.is_none() {
                current.song = current.songs.first().cloned();
            } else if let Some(song) = current.song.as_mut() {
                normalize_song(song);
            }
        }

        for schedule in &mut data.sched_next {
            for song in &mut schedule.songs {
                normalize_song(song);
            }
            if schedule.song.is_none() {
                schedule.song = schedule.songs.first().cloned();
            } else if let Some(song) = schedule.song.as_mut() {
                normalize_song(song);
            }
        }

        Ok(data)
    }

    pub async fn get_all_albums(&self) -> Result<Vec<Album>> {
        let url = format!("{}/all_albums", API_BASE);
        let params = self.get_params();

        #[derive(Deserialize)]
        struct AlbumsResult {
            success: bool,
            albums: Option<Vec<Album>>,
        }

        #[derive(Deserialize)]
        struct AlbumsResponse {
            albums: AlbumsResult,
        }

        let resp = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?
            .json::<AlbumsResponse>()
            .await?;

        if resp.albums.success {
            Ok(resp.albums.albums.unwrap_or_default())
        } else {
            Err(anyhow!("Failed to get albums"))
        }
    }

    pub async fn get_album(&self, album_id: i32) -> Result<Album> {
        let (album, _) = self.get_album_with_songs(album_id).await?;
        Ok(album)
    }

    pub async fn get_album_with_songs(&self, album_id: i32) -> Result<(Album, Vec<Song>)> {
        let url = format!("{}/album", API_BASE);
        let mut params = self.get_params();
        params.push(("album_id", album_id.to_string()));

        #[derive(Deserialize)]
        struct AlbumResult {
            success: bool,
            text: Option<String>,
            #[serde(default, alias = "song_data", alias = "songs")]
            songs: Vec<Song>,
        }

        #[derive(Deserialize)]
        struct AlbumResponse {
            album: AlbumResult,
            id: Option<i32>,
            name: Option<String>,
            artist: Option<String>,
            year: Option<i32>,
            rating: Option<f32>,
            rating_user: Option<f32>,
            fav: Option<bool>,
            song_count: Option<i32>,
            #[serde(default, alias = "song_data", alias = "songs")]
            songs: Vec<Song>,
        }

        let resp = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?
            .json::<AlbumResponse>()
            .await?;

        if resp.album.success {
            let album = Album {
                id: resp.id.unwrap_or(album_id),
                name: resp.name.unwrap_or_default(),
                artist: resp.artist.unwrap_or_default(),
                year: resp.year,
                rating: resp.rating,
                rating_user: resp.rating_user,
                fav: resp.fav,
                song_count: resp.song_count,
            };
            let mut songs = if resp.songs.is_empty() {
                resp.album.songs
            } else {
                resp.songs
            };
            for song in &mut songs {
                normalize_song(song);
            }
            Ok((album, songs))
        } else {
            Err(anyhow!(resp.album.text.unwrap_or_else(|| "Failed to get album".to_string())))
        }
    }

    pub async fn vote(&self, entry_id: i32) -> Result<()> {
        let url = format!("{}/vote", API_BASE);
        let params = self.get_params();

        #[derive(Deserialize)]
        struct Result {
            success: bool,
            text: Option<String>,
        }

        let resp = self.client
            .post(&url)
            .query(&params)
            .form(&[("entry_id", entry_id.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("result").and_then(|r| r.get("success")).and_then(|s| s.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = resp.get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Vote failed")
                .to_string();
            Err(anyhow!(msg))
        }
    }

    pub async fn request_song(&self, song_id: i32) -> Result<()> {
        let url = format!("{}/request", API_BASE);
        let params = self.get_params();

        let resp = self.client
            .post(&url)
            .query(&params)
            .form(&[("song_id", song_id.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("result").and_then(|r| r.get("success")).and_then(|s| s.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = resp.get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Request failed")
                .to_string();
            Err(anyhow!(msg))
        }
    }

    pub async fn delete_request(&self, request_id: i32) -> Result<()> {
        let url = format!("{}/delete_request", API_BASE);
        let params = self.get_params();

        let resp = self.client
            .post(&url)
            .query(&params)
            .form(&[("request_id", request_id.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("result").and_then(|r| r.get("success")).and_then(|s| s.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = resp.get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Delete request failed")
                .to_string();
            Err(anyhow!(msg))
        }
    }

    pub async fn rate_song(&self, song_id: i32, rating: f32) -> Result<()> {
        let url = format!("{}/rate", API_BASE);
        let params = self.get_params();

        let resp = self.client
            .post(&url)
            .query(&params)
            .form(&[("song_id", song_id.to_string()), ("rating", rating.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("result").and_then(|r| r.get("success")).and_then(|s| s.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = resp.get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Rating failed")
                .to_string();
            Err(anyhow!(msg))
        }
    }

    pub async fn fave_song(&self, song_id: i32, fave: bool) -> Result<()> {
        let url = format!("{}/fave_song", API_BASE);
        let params = self.get_params();

        let resp = self.client
            .post(&url)
            .query(&params)
            .form(&[("song_id", song_id.to_string()), ("fave", fave.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp.get("result").and_then(|r| r.get("success")).and_then(|s| s.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = resp.get("result")
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Fave failed")
                .to_string();
            Err(anyhow!(msg))
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Song>> {
        let url = format!("{}/search", API_BASE);
        let mut params = self.get_params();
        params.push(("query", query.to_string()));

        #[derive(Deserialize)]
        struct SearchResult {
            success: bool,
            results: Option<Vec<Song>>,
        }

        #[derive(Deserialize)]
        struct SearchResponse {
            search: SearchResult,
        }

        let resp = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;

        if resp.search.success {
            Ok(resp.search.results.unwrap_or_default())
        } else {
            Err(anyhow!("Search failed"))
        }
    }

    pub async fn get_stream_url(&self, format: &str) -> Result<String> {
        Ok(format!("https://rainwave.cc/tune_in/{}.{}", self.station_id, format))
    }

    pub async fn get_album_art(&self, url: &str, width: u32, height: u32) -> Result<AlbumArt> {
        let bytes = self.client
            .get(url)
            .send()
            .await?
            .bytes()
            .await?;
        let image = image::load_from_memory(&bytes)?
            .resize_exact(width, height, FilterType::Triangle)
            .to_rgb8();
        let pixels = image.pixels()
            .map(|p| (p[0], p[1], p[2]))
            .collect();

        Ok(AlbumArt { width, height, pixels })
    }
}

fn normalize_song(song: &mut Song) {
    if song.artist.is_empty() && !song.artists.is_empty() {
        song.artist = song.artists.iter()
            .map(|artist| artist.name.as_str())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }

    if song.album.is_none() {
        song.album = song.albums.first()
            .map(|album| album.name.clone())
            .filter(|name| !name.is_empty());
    }

    if song.art_url.is_none() {
        song.art_url = song.albums.first()
            .and_then(|album| album.art.clone())
            .map(|art| {
                if art.starts_with("http") {
                    art
                } else {
                    format!("https://rainwave.cc{}", art)
                }
            });
    }
}
