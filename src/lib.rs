use anyhow::{Context, Result, anyhow};
use image::GenericImageView;
use keyring::Entry;
use reqwest::blocking::Client;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub const BASE_URL: &str = "https://rainwave.cc";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Keyboard layout ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyLayout {
    Qwerty,
    Dvorak,
    Colemak,
}

impl KeyLayout {
    pub const ALL: &'static [KeyLayout] = &[
        KeyLayout::Qwerty,
        KeyLayout::Dvorak,
        KeyLayout::Colemak,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Qwerty => "QWERTY",
            Self::Dvorak => "Dvorak",
            Self::Colemak => "Colemak",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Qwerty => Self::Dvorak,
            Self::Dvorak => Self::Colemak,
            Self::Colemak => Self::Qwerty,
        }
    }
}

impl Default for KeyLayout {
    fn default() -> Self {
        Self::Qwerty
    }
}

pub struct KeyBindings {
    pub quit: char,
    pub rain_toggle: char,
    pub down: char,
    pub up: char,
    pub vote: char,
    pub request: char,
    pub delete: char,
    pub clear: char,
    pub fave: char,
    pub search: char,
    pub album: char,
    pub login: char,
    pub logout: char,
    pub history: char,
    pub copy_info: char,
    pub detail: char,
}

impl KeyBindings {
    pub fn for_layout(layout: KeyLayout) -> Self {
        match layout {
            KeyLayout::Qwerty => Self {
                quit: 'q',
                rain_toggle: 'r',
                down: 'j',
                up: 'k',
                vote: 'v',
                request: 'x',
                delete: 'd',
                clear: 'c',
                fave: 'f',
                search: '/',
                album: 'a',
                login: 'l',
                logout: 'o',
                history: 'h',
                copy_info: 'y',
                detail: 'i',
            },
            KeyLayout::Dvorak => Self {
                quit: '\'',
                rain_toggle: 'p',
                down: 'h',
                up: 't',
                vote: '.',
                request: 'q',
                delete: 'e',
                clear: 'i',
                fave: 'u',
                search: 'z',
                album: 'a',
                login: 'n',
                logout: 'r',
                history: 'b',
                copy_info: 'f',
                detail: 'c',
            },
            KeyLayout::Colemak => Self {
                quit: 'q',
                rain_toggle: 'p',
                down: 'n',
                up: 'e',
                vote: 'v',
                request: 'x',
                delete: 's',
                clear: 'i',
                fave: 't',
                search: '/',
                album: 'a',
                login: 'i',
                logout: 'y',
                history: 'h',
                copy_info: 'j',
                detail: 'u',
            },
        }
    }
}

// ── Config persistence ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme_idx: usize,
    pub refresh_interval: u64,
    pub mpv_binary: String,
    pub rain_show: bool,
    pub volume: u8,
    pub last_station: u8,
    pub layout: KeyLayout,
    pub notifications: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme_idx: 0,
            refresh_interval: 15,
            mpv_binary: "mpv".into(),
            rain_show: true,
            volume: 100,
            last_station: 1,
            layout: KeyLayout::Qwerty,
            notifications: true,
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        let base = dirs_or_home();
        base.join("rainwave-tui").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

// ── Station definitions ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Station {
    pub id: u8,
    pub name: &'static str,
    pub stream: &'static str,
    pub description: &'static str,
}

pub const STATIONS: [Station; 6] = [
    Station {
        id: 1,
        name: "Game",
        stream: "https://relay.rainwave.cc/game.mp3",
        description: "Video game soundtracks and original scores",
    },
    Station {
        id: 2,
        name: "OC ReMix",
        stream: "https://relay.rainwave.cc/ocremix.mp3",
        description: "OverClocked ReMix community arrangements",
    },
    Station {
        id: 3,
        name: "Covers",
        stream: "https://relay.rainwave.cc/covers.mp3",
        description: "Fan covers and arrangements of game music",
    },
    Station {
        id: 4,
        name: "Chiptunes",
        stream: "https://relay.rainwave.cc/chiptune.mp3",
        description: "8-bit, 16-bit, and chip-style original music",
    },
    Station {
        id: 5,
        name: "All",
        stream: "https://relay.rainwave.cc/all.mp3",
        description: "All stations combined into one stream",
    },
    Station {
        id: 6,
        name: "Chill",
        stream: "https://relay.rainwave.cc/chill.mp3",
        description: "Relaxing and ambient game music",
    },
];

pub fn station(id: u8) -> Station {
    STATIONS
        .iter()
        .copied()
        .find(|s| s.id == id)
        .unwrap_or(STATIONS[0])
}

// ── API data types ────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ApiInfo {
    pub time: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Artist {
    pub name: String,
    #[serde(default)]
    pub id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Album {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub art: Option<String>,
    #[serde(default, deserialize_with = "de_f64_opt")]
    pub rating: Option<f64>,
    #[serde(default)]
    pub fave: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Song {
    pub id: u64,
    pub title: String,
    #[serde(default, deserialize_with = "de_f64_opt")]
    pub rating: Option<f64>,
    #[serde(default, deserialize_with = "de_u64_opt")]
    pub length: Option<u64>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub entry_id: Option<u64>,
    #[serde(default)]
    pub entry_votes: Option<u64>,
    #[serde(default)]
    pub rating_user: Option<f64>,
    #[serde(default)]
    pub fave: Option<bool>,
}

impl Song {
    pub fn artists_text(&self) -> String {
        let names = self
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>();
        if names.is_empty() {
            "Unknown artist".into()
        } else {
            names.join(", ")
        }
    }

    pub fn album_name(&self) -> String {
        self.albums
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Unknown album".into())
    }

    pub fn art_url(&self) -> Option<String> {
        self.albums
            .first()
            .and_then(|a| a.art.as_ref())
            .map(|path| {
                if path.starts_with("http") {
                    path.clone()
                } else {
                    format!("{BASE_URL}{path}")
                }
            })
    }

    pub fn length_text(&self) -> String {
        match self.length {
            Some(len) => format!("{:02}:{:02}", len / 60, len % 60),
            None => "--:--".into(),
        }
    }

    pub fn rating_text(&self) -> String {
        match self.rating {
            Some(r) => format!("{r:.1}"),
            None => "-.--".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Schedule {
    pub start: i64,
    pub end: i64,
    #[serde(default)]
    pub voting_allowed: bool,
    #[serde(default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RequestLineItem {
    pub username: String,
    pub position: u64,
    #[serde(default)]
    pub song: Option<RequestSong>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RequestSong {
    pub title: String,
    pub album_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InfoResponse {
    pub api_info: ApiInfo,
    pub sched_current: Schedule,
    #[serde(default)]
    pub sched_next: Vec<Schedule>,
    #[serde(default)]
    pub request_line: Vec<RequestLineItem>,
    #[serde(default)]
    pub all_stations_info: HashMap<String, StationNow>,
}

impl InfoResponse {
    pub fn current_song(&self) -> Option<&Song> {
        self.sched_current.songs.first()
    }

    pub fn progress(&self) -> f64 {
        let total = (self.sched_current.end - self.sched_current.start).max(1) as f64;
        let elapsed =
            (self.api_info.time - self.sched_current.start).clamp(0, total as i64) as f64;
        elapsed / total
    }

    pub fn elapsed_secs(&self) -> i64 {
        (self.api_info.time - self.sched_current.start).max(0)
    }

    pub fn remaining_secs(&self) -> i64 {
        (self.sched_current.end - self.api_info.time).max(0)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StationNow {
    pub title: String,
    pub album: String,
    pub artists: String,
    pub art: String,
    pub event_type: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SearchResponse {
    #[serde(default)]
    pub songs: Vec<Song>,
    #[serde(default)]
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AlbumResponse {
    #[serde(default)]
    pub album: Option<Album>,
    #[serde(default)]
    pub songs: Vec<Song>,
}

// ── History ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HistoryItem {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub rating: Option<f64>,
    pub station: String,
    pub length: Option<u64>,
}

// ── Credentials / keyring ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Credentials {
    pub user_id: String,
    pub api_key: String,
}

pub struct SecretStore;

impl SecretStore {
    const SERVICE: &'static str = "rainwave-tui";

    pub fn save(creds: &Credentials) -> Result<()> {
        Entry::new(Self::SERVICE, "user_id")?.set_password(&creds.user_id)?;
        Entry::new(Self::SERVICE, "api_key")?.set_password(&creds.api_key)?;
        Ok(())
    }

    pub fn load() -> Option<Credentials> {
        let user_id = Entry::new(Self::SERVICE, "user_id")
            .ok()?
            .get_password()
            .ok()?;
        let api_key = Entry::new(Self::SERVICE, "api_key")
            .ok()?
            .get_password()
            .ok()?;
        Some(Credentials { user_id, api_key })
    }

    pub fn clear() -> Result<()> {
        let _ = Entry::new(Self::SERVICE, "user_id")?.delete_credential();
        let _ = Entry::new(Self::SERVICE, "api_key")?.delete_credential();
        Ok(())
    }
}

// ── HTTP client ───────────────────────────────────────────────

pub struct RainwaveClient {
    http: Client,
    creds: Option<Credentials>,
}

impl RainwaveClient {
    pub fn new(creds: Option<Credentials>) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent(format!("rainwave-tui/{VERSION}"))
            .build()?;
        Ok(Self { http, creds })
    }

    pub fn info(&self, sid: u8) -> Result<InfoResponse> {
        self.post("info", sid, &[])
    }
    pub fn search(&self, sid: u8, query: &str) -> Result<SearchResponse> {
        self.post("search", sid, &[("search", query)])
    }
    pub fn album(&self, sid: u8, id: u64) -> Result<AlbumResponse> {
        self.post("album", sid, &[("id", &id.to_string())])
    }
    pub fn vote(&self, sid: u8, entry_id: u64) -> Result<()> {
        self.post_unit("vote", sid, &[("entry_id", &entry_id.to_string())])
    }
    pub fn request(&self, sid: u8, song_id: u64) -> Result<()> {
        self.post_unit("request", sid, &[("song_id", &song_id.to_string())])
    }
    pub fn delete_request(&self, sid: u8, song_id: u64) -> Result<()> {
        self.post_unit("delete_request", sid, &[("song_id", &song_id.to_string())])
    }
    pub fn rate(&self, sid: u8, song_id: u64, rating: f64) -> Result<()> {
        self.post_unit(
            "rate",
            sid,
            &[
                ("song_id", &song_id.to_string()),
                ("rating", &format!("{rating:.1}")),
            ],
        )
    }
    pub fn fave_song(&self, sid: u8, song_id: u64, fave: bool) -> Result<()> {
        self.post_unit(
            "fave_song",
            sid,
            &[
                ("song_id", &song_id.to_string()),
                ("fave", if fave { "true" } else { "false" }),
            ],
        )
    }
    pub fn clear_requests(&self, sid: u8) -> Result<()> {
        self.post_unit("clear_requests", sid, &[])
    }

    pub fn fetch_album_art(
        &self,
        url: &str,
        cols: u32,
        rows: u32,
    ) -> Result<Vec<Vec<(u8, u8, u8)>>> {
        let bytes = self.http.get(url).send()?.error_for_status()?.bytes()?;
        let img = image::load_from_memory(&bytes)?.resize_exact(
            cols,
            rows,
            image::imageops::FilterType::Triangle,
        );
        let mut pixels = Vec::new();
        for y in 0..rows {
            let mut row = Vec::new();
            for x in 0..cols {
                let p = img.get_pixel(x, y);
                row.push((p[0], p[1], p[2]));
            }
            pixels.push(row);
        }
        Ok(pixels)
    }

    fn post<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        sid: u8,
        extra: &[(&str, &str)],
    ) -> Result<T> {
        self.post_impl(endpoint, sid, extra)
    }

    fn post_unit(&self, endpoint: &str, sid: u8, extra: &[(&str, &str)]) -> Result<()> {
        let _: serde_json::Value = self.post_impl(endpoint, sid, extra)?;
        Ok(())
    }

    fn post_impl<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        sid: u8,
        extra: &[(&str, &str)],
    ) -> Result<T> {
        let mut form = vec![("sid", sid.to_string())];
        if let Some(creds) = &self.creds {
            form.push(("user", creds.user_id.clone()));
            form.push(("key", creds.api_key.clone()));
        }
        for (key, value) in extra {
            form.push((*key, (*value).to_string()));
        }
        self.http
            .post(format!("{BASE_URL}/api4/{endpoint}"))
            .form(&form)
            .send()
            .with_context(|| format!("Rainwave API request failed: {endpoint}"))?
            .error_for_status()?
            .json::<T>()
            .with_context(|| format!("Rainwave API returned unexpected data for {endpoint}"))
    }
}

// ── MPV player ────────────────────────────────────────────────

pub struct Player {
    child: Option<Child>,
    volume: u8,
    mpv_binary: String,
}

impl Player {
    pub fn new(volume: u8, mpv_binary: String) -> Self {
        Self {
            child: None,
            volume,
            mpv_binary,
        }
    }
    pub fn volume(&self) -> u8 {
        self.volume
    }
    pub fn set_volume(&mut self, vol: u8) {
        self.volume = vol.min(150);
    }
    pub fn is_playing(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            if child.try_wait().ok().flatten().is_some() {
                self.child = None;
            }
        }
        self.child.is_some()
    }
    pub fn play(&mut self, station: Station) -> Result<()> {
        self.stop();
        let child = Command::new(&self.mpv_binary)
            .arg("--no-video")
            .arg("--really-quiet")
            .arg(format!("--volume={}", self.volume))
            .arg(station.stream)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to start mpv; install mpv or keep playback paused")?;
        self.child = Some(child);
        Ok(())
    }
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    pub fn toggle(&mut self, station: Station) -> Result<()> {
        if self.is_playing() {
            self.stop();
            Ok(())
        } else {
            self.play(station)
        }
    }
    pub fn restart(&mut self, station: Station) -> Result<()> {
        if self.is_playing() {
            self.play(station)?;
        }
        Ok(())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Utilities ─────────────────────────────────────────────────

pub fn notify(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .arg("--icon=audio-x-generic")
        .arg("-a")
        .arg("rainwave-tui")
        .arg(summary)
        .arg(body)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    if let Ok(mut child) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return Ok(());
    }
    if let Ok(mut child) = Command::new("xsel")
        .arg("--clipboard")
        .arg("--input")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return Ok(());
    }
    Err(anyhow!("no clipboard tool found (install xclip or xsel)"))
}

pub fn parse_info(json: &str) -> Result<InfoResponse> {
    serde_json::from_str(json).map_err(|e| anyhow!(e))
}

// ── Serde helpers ─────────────────────────────────────────────

fn de_f64_opt<'de, D>(deserializer: D) -> std::result::Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    })
}

fn de_u64_opt<'de, D>(deserializer: D) -> std::result::Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    })
}

// ── Rating helpers ────────────────────────────────────────────

pub fn rating_stars(rating: f64) -> (String, Color) {
    let full = rating.floor() as u32;
    let half = (rating - rating.floor()) >= 0.25;
    let empty = 5 - full - if half { 1 } else { 0 };
    let mut s = String::new();
    for _ in 0..full {
        s.push('★');
    }
    if half {
        s.push('★');
    }
    for _ in 0..empty {
        s.push('☆');
    }
    let color = if rating >= 4.5 {
        Color::LightGreen
    } else if rating >= 3.5 {
        Color::LightYellow
    } else if rating >= 2.5 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    (s, color)
}

use ratatui::style::Color;

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_info_response_and_progress() {
        let json = r#"{
            "api_info":{"time":110},
            "sched_current":{"start":100,"end":200,"voting_allowed":false,"songs":[{
                "id":42,"title":"Skyline","rating":4.2,"length":180,
                "artists":[{"name":"A Composer"}],
                "albums":[{"id":7,"name":"Blue Album","art":"/album_art/1_7","rating":"4.5","fave":true}],
                "entry_id":99,"entry_votes":3,"rating_user":4.0,"fave":false
            }]},
            "sched_next":[{"start":200,"end":300,"voting_allowed":true,"songs":[{"id":43,"title":"Vote Me","artists":[],"albums":[],"entry_id":100}]}],
            "request_line":[{"username":"alice","position":1,"song":{"title":"Req","album_name":"Queue Album"}}],
            "all_stations_info":{"1":{"title":"Skyline","album":"Blue Album","artists":"A Composer","art":"/album_art/1_7","event_type":"Election"}}
        }"#;
        let info = parse_info(json).unwrap();
        let song = info.current_song().unwrap();
        assert_eq!(song.title, "Skyline");
        assert_eq!(song.artists_text(), "A Composer");
        assert_eq!(song.album_name(), "Blue Album");
        assert_eq!(
            song.art_url().unwrap(),
            "https://rainwave.cc/album_art/1_7"
        );
        assert!((info.progress() - 0.10).abs() < f64::EPSILON);
        assert!(info.sched_next[0].voting_allowed);
        assert_eq!(
            info.request_line[0].song.as_ref().unwrap().album_name,
            "Queue Album"
        );
    }

    #[test]
    fn tolerant_numeric_fields_accept_null_string_or_number() {
        let json = r#"{
            "api_info":{"time":0},
            "sched_current":{"start":0,"end":1,"songs":[{"id":1,"title":"A","rating":"3.8","length":"64","artists":[],"albums":[{"id":1,"name":"X","rating":null}]}]}
        }"#;
        let info = parse_info(json).unwrap();
        let song = info.current_song().unwrap();
        assert_eq!(song.rating, Some(3.8));
        assert_eq!(song.length, Some(64));
        assert_eq!(song.albums[0].rating, None);
    }

    #[test]
    fn keybindings_presets_have_distinct_keys() {
        for layout in KeyLayout::ALL {
            let kb = KeyBindings::for_layout(*layout);
            let mut keys: Vec<char> = vec![
                kb.quit,
                kb.rain_toggle,
                kb.down,
                kb.up,
                kb.vote,
                kb.request,
                kb.delete,
                kb.clear,
                kb.fave,
                kb.history,
                kb.copy_info,
                kb.detail,
            ];
            keys.sort();
            keys.dedup();
            assert_eq!(
                keys.len(),
                12,
                "Layout {:?} has duplicate bindings",
                layout
            );
        }
    }
}
