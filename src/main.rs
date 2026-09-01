use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rainwave_tui::{
    Album, Config, HistoryItem, InfoResponse, KeyBindings, KeyLayout, Player, RainwaveClient,
    STATIONS, SearchResponse, SecretStore, Station, VERSION, copy_to_clipboard, notify, station,
};
use rand::Rng;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::{
    io,
    time::{Duration, Instant},
};

const HISTORY_MAX: usize = 50;

// ── Rain effect ───────────────────────────────────────────────

const RAIN_CHARS: &[char] = &['│', '╎', '┆', '·', ':', '┊', '╽', '∣'];
const RAIN_COUNT: usize = 60;

struct Raindrop {
    x: f64,
    y: f64,
    speed: f64,
    ch: char,
    bright: f64,
}

impl Raindrop {
    fn new(rng: &mut impl Rng, w: f64) -> Self {
        Self {
            x: rng.gen_range(0.0..w),
            y: rng.gen_range(-30.0..0.0),
            speed: rng.gen_range(0.3..1.4),
            ch: RAIN_CHARS[rng.gen_range(0..RAIN_CHARS.len())],
            bright: rng.gen_range(0.12..0.45),
        }
    }
    fn tick(&mut self, rng: &mut impl Rng, w: f64, h: f64) {
        self.y += self.speed;
        if self.y >= h {
            self.y = -2.0;
            self.x = rng.gen_range(0.0..w);
            self.speed = rng.gen_range(0.3..1.4);
            self.ch = RAIN_CHARS[rng.gen_range(0..RAIN_CHARS.len())];
            self.bright = rng.gen_range(0.12..0.45);
        }
    }
}

// ── Theme system ──────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Theme {
    name: &'static str,
    accent: Color,
    highlight_fg: Color,
    highlight_bg: Color,
    now_playing: Color,
    artist: Color,
    album: Color,
    border_primary: Color,
    border_secondary: Color,
    border_vote: Color,
    border_search: Color,
    border_req: Color,
    border_album: Color,
    border_login: Color,
    border_art: Color,
    border_history: Color,
    border_info: Color,
    gauge_fg: Color,
    gauge_bg: Color,
    help_fg: Color,
    help_bg: Color,
    dim: Color,
    success: Color,
    error: Color,
}

const THEMES: &[Theme] = &[
    Theme {
        name: "Ocean",
        accent: Color::LightCyan,
        highlight_fg: Color::Black,
        highlight_bg: Color::LightCyan,
        now_playing: Color::LightMagenta,
        artist: Color::LightCyan,
        album: Color::LightYellow,
        border_primary: Color::LightBlue,
        border_secondary: Color::Cyan,
        border_vote: Color::LightMagenta,
        border_search: Color::Yellow,
        border_req: Color::Green,
        border_album: Color::LightYellow,
        border_login: Color::Blue,
        border_art: Color::Magenta,
        border_history: Color::LightGreen,
        border_info: Color::LightBlue,
        gauge_fg: Color::LightCyan,
        gauge_bg: Color::DarkGray,
        help_fg: Color::White,
        help_bg: Color::Black,
        dim: Color::DarkGray,
        success: Color::LightGreen,
        error: Color::LightRed,
    },
    Theme {
        name: "Dracula",
        accent: Color::Rgb(189, 147, 249),
        highlight_fg: Color::Rgb(40, 42, 54),
        highlight_bg: Color::Rgb(189, 147, 249),
        now_playing: Color::Rgb(255, 121, 198),
        artist: Color::Rgb(139, 233, 253),
        album: Color::Rgb(241, 250, 140),
        border_primary: Color::Rgb(80, 250, 123),
        border_secondary: Color::Rgb(189, 147, 249),
        border_vote: Color::Rgb(255, 121, 198),
        border_search: Color::Rgb(241, 250, 140),
        border_req: Color::Rgb(80, 250, 123),
        border_album: Color::Rgb(255, 184, 108),
        border_login: Color::Rgb(139, 233, 253),
        border_art: Color::Rgb(255, 121, 198),
        border_history: Color::Rgb(80, 250, 123),
        border_info: Color::Rgb(189, 147, 249),
        gauge_fg: Color::Rgb(80, 250, 123),
        gauge_bg: Color::Rgb(68, 71, 90),
        help_fg: Color::Rgb(248, 248, 242),
        help_bg: Color::Rgb(40, 42, 54),
        dim: Color::Rgb(98, 114, 164),
        success: Color::Rgb(80, 250, 123),
        error: Color::Rgb(255, 85, 85),
    },
    Theme {
        name: "Nord",
        accent: Color::Rgb(136, 192, 208),
        highlight_fg: Color::Rgb(46, 52, 64),
        highlight_bg: Color::Rgb(136, 192, 208),
        now_playing: Color::Rgb(191, 97, 106),
        artist: Color::Rgb(136, 192, 208),
        album: Color::Rgb(163, 190, 140),
        border_primary: Color::Rgb(94, 129, 172),
        border_secondary: Color::Rgb(129, 161, 193),
        border_vote: Color::Rgb(191, 97, 106),
        border_search: Color::Rgb(235, 203, 139),
        border_req: Color::Rgb(163, 190, 140),
        border_album: Color::Rgb(208, 135, 112),
        border_login: Color::Rgb(136, 192, 208),
        border_art: Color::Rgb(180, 142, 173),
        border_history: Color::Rgb(163, 190, 140),
        border_info: Color::Rgb(94, 129, 172),
        gauge_fg: Color::Rgb(94, 129, 172),
        gauge_bg: Color::Rgb(59, 66, 82),
        help_fg: Color::Rgb(216, 222, 233),
        help_bg: Color::Rgb(46, 52, 64),
        dim: Color::Rgb(76, 86, 106),
        success: Color::Rgb(163, 190, 140),
        error: Color::Rgb(191, 97, 106),
    },
    Theme {
        name: "Monokai",
        accent: Color::Rgb(166, 226, 46),
        highlight_fg: Color::Rgb(39, 40, 34),
        highlight_bg: Color::Rgb(166, 226, 46),
        now_playing: Color::Rgb(249, 38, 114),
        artist: Color::Rgb(102, 217, 239),
        album: Color::Rgb(230, 219, 116),
        border_primary: Color::Rgb(166, 226, 46),
        border_secondary: Color::Rgb(102, 217, 239),
        border_vote: Color::Rgb(249, 38, 114),
        border_search: Color::Rgb(230, 219, 116),
        border_req: Color::Rgb(166, 226, 46),
        border_album: Color::Rgb(253, 151, 31),
        border_login: Color::Rgb(102, 217, 239),
        border_art: Color::Rgb(249, 38, 114),
        border_history: Color::Rgb(166, 226, 46),
        border_info: Color::Rgb(102, 217, 239),
        gauge_fg: Color::Rgb(166, 226, 46),
        gauge_bg: Color::Rgb(73, 72, 62),
        help_fg: Color::Rgb(248, 248, 242),
        help_bg: Color::Rgb(39, 40, 34),
        dim: Color::Rgb(117, 113, 94),
        success: Color::Rgb(166, 226, 46),
        error: Color::Rgb(249, 38, 114),
    },
    Theme {
        name: "Solarized Dark",
        accent: Color::Rgb(38, 139, 210),
        highlight_fg: Color::Rgb(0, 43, 54),
        highlight_bg: Color::Rgb(38, 139, 210),
        now_playing: Color::Rgb(220, 50, 47),
        artist: Color::Rgb(38, 139, 210),
        album: Color::Rgb(133, 153, 0),
        border_primary: Color::Rgb(38, 139, 210),
        border_secondary: Color::Rgb(108, 113, 196),
        border_vote: Color::Rgb(220, 50, 47),
        border_search: Color::Rgb(181, 137, 0),
        border_req: Color::Rgb(133, 153, 0),
        border_album: Color::Rgb(203, 75, 22),
        border_login: Color::Rgb(38, 139, 210),
        border_art: Color::Rgb(211, 54, 130),
        border_history: Color::Rgb(133, 153, 0),
        border_info: Color::Rgb(108, 113, 196),
        gauge_fg: Color::Rgb(38, 139, 210),
        gauge_bg: Color::Rgb(7, 54, 66),
        help_fg: Color::Rgb(147, 161, 161),
        help_bg: Color::Rgb(0, 43, 54),
        dim: Color::Rgb(88, 110, 117),
        success: Color::Rgb(133, 153, 0),
        error: Color::Rgb(220, 50, 47),
    },
    Theme {
        name: "Catppuccin Mocha",
        accent: Color::Rgb(137, 180, 250),
        highlight_fg: Color::Rgb(30, 30, 46),
        highlight_bg: Color::Rgb(137, 180, 250),
        now_playing: Color::Rgb(243, 139, 168),
        artist: Color::Rgb(137, 180, 250),
        album: Color::Rgb(249, 226, 175),
        border_primary: Color::Rgb(166, 227, 161),
        border_secondary: Color::Rgb(137, 180, 250),
        border_vote: Color::Rgb(243, 139, 168),
        border_search: Color::Rgb(249, 226, 175),
        border_req: Color::Rgb(166, 227, 161),
        border_album: Color::Rgb(250, 179, 135),
        border_login: Color::Rgb(137, 180, 250),
        border_art: Color::Rgb(203, 166, 247),
        border_history: Color::Rgb(166, 227, 161),
        border_info: Color::Rgb(137, 180, 250),
        gauge_fg: Color::Rgb(166, 227, 161),
        gauge_bg: Color::Rgb(49, 50, 68),
        help_fg: Color::Rgb(205, 214, 244),
        help_bg: Color::Rgb(30, 30, 46),
        dim: Color::Rgb(108, 112, 134),
        success: Color::Rgb(166, 227, 161),
        error: Color::Rgb(243, 139, 168),
    },
    Theme {
        name: "Gruvbox",
        accent: Color::Rgb(180, 95, 6),
        highlight_fg: Color::Rgb(40, 40, 40),
        highlight_bg: Color::Rgb(250, 189, 47),
        now_playing: Color::Rgb(254, 128, 25),
        artist: Color::Rgb(131, 165, 152),
        album: Color::Rgb(215, 153, 79),
        border_primary: Color::Rgb(184, 187, 38),
        border_secondary: Color::Rgb(131, 165, 152),
        border_vote: Color::Rgb(250, 189, 47),
        border_search: Color::Rgb(215, 153, 79),
        border_req: Color::Rgb(184, 187, 38),
        border_album: Color::Rgb(254, 128, 25),
        border_login: Color::Rgb(131, 165, 152),
        border_art: Color::Rgb(214, 93, 14),
        border_history: Color::Rgb(184, 187, 38),
        border_info: Color::Rgb(131, 165, 152),
        gauge_fg: Color::Rgb(184, 187, 38),
        gauge_bg: Color::Rgb(60, 56, 54),
        help_fg: Color::Rgb(235, 219, 178),
        help_bg: Color::Rgb(40, 40, 40),
        dim: Color::Rgb(124, 111, 100),
        success: Color::Rgb(184, 187, 38),
        error: Color::Rgb(250, 189, 47),
    },
    Theme {
        name: "Tokyo Night",
        accent: Color::Rgb(125, 167, 221),
        highlight_fg: Color::Rgb(36, 40, 59),
        highlight_bg: Color::Rgb(125, 167, 221),
        now_playing: Color::Rgb(187, 154, 247),
        artist: Color::Rgb(122, 165, 231),
        album: Color::Rgb(224, 175, 104),
        border_primary: Color::Rgb(122, 162, 247),
        border_secondary: Color::Rgb(125, 167, 221),
        border_vote: Color::Rgb(187, 154, 247),
        border_search: Color::Rgb(224, 175, 104),
        border_req: Color::Rgb(158, 206, 106),
        border_album: Color::Rgb(255, 158, 100),
        border_login: Color::Rgb(125, 167, 221),
        border_art: Color::Rgb(192, 115, 222),
        border_history: Color::Rgb(158, 206, 106),
        border_info: Color::Rgb(122, 162, 247),
        gauge_fg: Color::Rgb(122, 162, 247),
        gauge_bg: Color::Rgb(56, 60, 82),
        help_fg: Color::Rgb(169, 177, 214),
        help_bg: Color::Rgb(36, 40, 59),
        dim: Color::Rgb(86, 95, 137),
        success: Color::Rgb(158, 206, 106),
        error: Color::Rgb(247, 118, 142),
    },
    Theme {
        name: "Rosé Pine",
        accent: Color::Rgb(224, 175, 104),
        highlight_fg: Color::Rgb(30, 30, 46),
        highlight_bg: Color::Rgb(224, 175, 104),
        now_playing: Color::Rgb(235, 111, 146),
        artist: Color::Rgb(144, 202, 249),
        album: Color::Rgb(224, 175, 104),
        border_primary: Color::Rgb(49, 116, 143),
        border_secondary: Color::Rgb(31, 111, 135),
        border_vote: Color::Rgb(235, 111, 146),
        border_search: Color::Rgb(224, 175, 104),
        border_req: Color::Rgb(156, 207, 216),
        border_album: Color::Rgb(201, 165, 138),
        border_login: Color::Rgb(49, 116, 143),
        border_art: Color::Rgb(196, 167, 231),
        border_history: Color::Rgb(156, 207, 216),
        border_info: Color::Rgb(49, 116, 143),
        gauge_fg: Color::Rgb(49, 116, 143),
        gauge_bg: Color::Rgb(38, 38, 55),
        help_fg: Color::Rgb(224, 222, 244),
        help_bg: Color::Rgb(30, 30, 46),
        dim: Color::Rgb(110, 109, 134),
        success: Color::Rgb(156, 207, 216),
        error: Color::Rgb(235, 111, 146),
    },
];

fn theme(app: &App) -> &'static Theme {
    &THEMES[app.theme_idx]
}

// ── Sections ──────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Stations,
    Voting,
    Search,
    Requests,
    Albums,
    Login,
    History,
}

impl Section {
    fn next(self) -> Self {
        match self {
            Self::Stations => Self::Voting,
            Self::Voting => Self::Search,
            Self::Search => Self::Requests,
            Self::Requests => Self::Albums,
            Self::Albums => Self::History,
            Self::History => Self::Login,
            Self::Login => Self::Stations,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Stations => Self::Login,
            Self::Voting => Self::Stations,
            Self::Search => Self::Voting,
            Self::Requests => Self::Search,
            Self::Albums => Self::Requests,
            Self::History => Self::Albums,
            Self::Login => Self::History,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Stations => "Stations",
            Self::Voting => "Voting",
            Self::Search => "Search",
            Self::Requests => "Requests",
            Self::Albums => "Albums",
            Self::Login => "Login",
            Self::History => "History",
        }
    }
}

// ── Input modes ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Search,
    Album,
    LoginUser,
    LoginKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OptionsTab {
    Palette,
    General,
    Layout,
}

// ── App state ─────────────────────────────────────────────────

struct App {
    client: RainwaveClient,
    player: Player,
    sid: u8,
    info: Option<InfoResponse>,
    search: SearchResponse,
    albums: Vec<Album>,
    art: Vec<Vec<(u8, u8, u8)>>,
    section: Section,
    selected: usize,
    status: String,
    last_refresh: Instant,
    show_help: bool,
    activity: f64,
    logged_in: bool,
    input_mode: Option<InputMode>,
    input_buf: String,
    input_cursor: usize,
    login_user: String,
    art_cache: std::collections::HashMap<String, Vec<Vec<(u8, u8, u8)>>>,
    // Options
    show_options: bool,
    options_tab: OptionsTab,
    options_selected: usize,
    theme_idx: usize,
    refresh_interval: u64,
    mpv_binary: String,
    request_search: bool,
    // Rain
    raindrops: Vec<Raindrop>,
    rain_show: bool,
    term_w: f64,
    term_h: f64,
    // History
    history: Vec<HistoryItem>,
    last_history_title: String,
    // Config / layout
    config: Config,
    keys: KeyBindings,
    // Detail panel
    show_detail: bool,
    // Notifications
    notify_cooldown: Instant,
    // Session stats
    songs_heard: u32,
    session_start: Instant,
}

impl App {
    fn new() -> Result<Self> {
        let config = Config::load();
        let creds = SecretStore::load();
        let logged_in = creds.is_some();
        let keys = KeyBindings::for_layout(config.layout);
        let mut rng = rand::thread_rng();
        let raindrops: Vec<Raindrop> = (0..RAIN_COUNT)
            .map(|_| Raindrop::new(&mut rng, 80.0))
            .collect();
        Ok(Self {
            client: RainwaveClient::new(creds)?,
            player: Player::new(config.volume, config.mpv_binary.clone()),
            sid: config.last_station,
            info: None,
            search: SearchResponse::default(),
            albums: Vec::new(),
            art: Vec::new(),
            section: Section::Stations,
            selected: 0,
            status: format!("rainwave-tui v{VERSION} — Press ? for help"),
            last_refresh: Instant::now() - Duration::from_secs(60),
            show_help: false,
            request_search: false,
            activity: 0.0,
            logged_in,
            input_mode: None,
            input_buf: String::new(),
            input_cursor: 0,
            login_user: String::new(),
            art_cache: std::collections::HashMap::new(),
            show_options: false,
            options_tab: OptionsTab::Palette,
            options_selected: 0,
            theme_idx: config.theme_idx.min(THEMES.len() - 1),
            refresh_interval: config.refresh_interval,
            mpv_binary: config.mpv_binary.clone(),
            raindrops,
            rain_show: config.rain_show,
            term_w: 80.0,
            term_h: 24.0,
            history: Vec::new(),
            last_history_title: String::new(),
            config,
            keys,
            show_detail: false,
            notify_cooldown: Instant::now(),
            songs_heard: 0,
            session_start: Instant::now(),
        })
    }

    fn tick_rain(&mut self) {
        if !self.rain_show {
            return;
        }
        let mut rng = rand::thread_rng();
        for drop in &mut self.raindrops {
            drop.tick(&mut rng, self.term_w, self.term_h);
        }
    }

    fn refresh(&mut self) {
        match self.client.info(self.sid) {
            Ok(info) => {
                let art_url = info.current_song().and_then(|s| s.art_url());
                self.albums = info
                    .current_song()
                    .map(|s| s.albums.clone())
                    .unwrap_or_default();
                // Track history
                if let Some(song) = info.current_song() {
                    let title = song.title.clone();
                    if title != self.last_history_title && !title.is_empty() {
                        self.last_history_title = title.clone();
                        self.songs_heard += 1;
                        let station_name = station(self.sid).name;
                        self.history.insert(
                            0,
                            HistoryItem {
                                title: song.title.clone(),
                                artist: song.artists_text(),
                                album: song.album_name(),
                                rating: song.rating,
                                station: station_name.into(),
                                length: song.length,
                            },
                        );
                        self.history.truncate(HISTORY_MAX);
                        // Desktop notification
                        if self.config.notifications && self.notify_cooldown.elapsed().as_secs() > 3
                        {
                            notify(
                                &format!("Now Playing — {station_name}"),
                                &format!("{} — {}", song.artists_text(), song.title),
                            );
                            self.notify_cooldown = Instant::now();
                        }
                    }
                }
                self.info = Some(info);
                self.last_refresh = Instant::now();
                if let Some(url) = art_url {
                    if !self.art_cache.contains_key(&url) {
                        if let Ok(art) = self.client.fetch_album_art(&url, 14, 8) {
                            self.art_cache.insert(url.clone(), art);
                        }
                    }
                    self.art = self.art_cache.get(&url).cloned().unwrap_or_default();
                }
                self.status = format!("Updated {}", station(self.sid).name);
            }
            Err(err) => self.status = format!("Refresh failed: {err:#}"),
        }
    }

    fn save_config(&mut self) {
        self.config.theme_idx = self.theme_idx;
        self.config.refresh_interval = self.refresh_interval;
        self.config.mpv_binary = self.mpv_binary.clone();
        self.config.rain_show = self.rain_show;
        self.config.volume = self.player.volume();
        self.config.last_station = self.sid;
        if let Err(e) = self.config.save() {
            self.status = format!("Config save failed: {e:#}");
        }
    }

    fn current_station(&self) -> Station {
        station(self.sid)
    }

    fn switch_station(&mut self, sid: u8) {
        self.sid = sid;
        self.selected = 0;
        if self.player.is_playing() {
            if let Err(err) = self.player.play(self.current_station()) {
                self.status = format!("mpv: {err:#}");
            }
        }
        self.refresh();
    }

    fn section_hints(&self) -> String {
        let k = &self.keys;
        match self.section {
            Section::Stations => format!(
                "1-6: switch | Tab: next | {}/{}: select",
                k.up, k.down
            ),
            Section::Voting => format!(
                "{}/{}: select | Enter/v: vote | x: request",
                k.up, k.down
            ),
            Section::Search => format!(
                "{}/{}: select | x: request | Enter: again | /: new",
                k.up, k.down
            ),
            Section::Requests => {
                format!(
                    "{}/{}: select | /: search | d: delete | c: clear",
                    k.up, k.down
                )
            }
            Section::Albums => format!("{}/{}: browse | a: load by ID", k.up, k.down),
            Section::Login => format!("l: login | o: logout"),
            Section::History => format!("{}/{}: browse | Esc: back", k.up, k.down),
        }
    }

    fn active_items_len(&self) -> usize {
        match self.section {
            Section::Stations => STATIONS.len(),
            Section::Voting => self
                .info
                .as_ref()
                .and_then(|i| i.sched_next.first())
                .map(|s| s.songs.len())
                .unwrap_or(0),
            Section::Search => self.search.songs.len(),
            Section::Requests if self.request_search => self.search.songs.len(),
            Section::Requests => self
                .info
                .as_ref()
                .map(|i| i.request_line.len())
                .unwrap_or(0),
            Section::Albums => self.albums.len(),
            Section::Login => 1,
            Section::History => self.history.len(),
        }
    }

    fn selected_song_id(&self) -> Option<u64> {
        match self.section {
            Section::Search => self.search.songs.get(self.selected).map(|s| s.id),
            Section::Requests if self.request_search => {
                self.search.songs.get(self.selected).map(|s| s.id)
            }
            Section::Voting => self
                .info
                .as_ref()?
                .sched_next
                .first()?
                .songs
                .get(self.selected)
                .map(|s| s.id),
            _ => self.info.as_ref()?.current_song().map(|s| s.id),
        }
    }

    fn vote(&mut self) {
        let Some(entry_id) = self
            .info
            .as_ref()
            .and_then(|i| i.sched_next.first())
            .and_then(|e| e.songs.get(self.selected))
            .and_then(|s| s.entry_id)
        else {
            self.status = "No election entry selected.".into();
            return;
        };
        self.status = match self.client.vote(self.sid, entry_id) {
            Ok(()) => "Vote sent.".into(),
            Err(e) => format!("Vote failed: {e:#}"),
        };
    }

    fn request_selected(&mut self) {
        let Some(song_id) = self.selected_song_id() else {
            self.status = "No song selected.".into();
            return;
        };
        self.status = match self.client.request(self.sid, song_id) {
            Ok(()) => "Request queued.".into(),
            Err(e) => format!("Request failed: {e:#}"),
        };
    }

    fn rate_current(&mut self, rating: f64) {
        let Some(song_id) = self.selected_song_id() else {
            self.status = "No song selected.".into();
            return;
        };
        self.status = match self.client.rate(self.sid, song_id, rating) {
            Ok(()) => format!("Rated {rating:.1}."),
            Err(e) => format!("Rating failed: {e:#}"),
        };
    }

    fn fave_current(&mut self) {
        let Some(song) = self.info.as_ref().and_then(|i| i.current_song()) else {
            self.status = "No song playing.".into();
            return;
        };
        let fave = !song.fave.unwrap_or(false);
        self.status = match self.client.fave_song(self.sid, song.id, fave) {
            Ok(()) => {
                if fave {
                    "Favorited.".into()
                } else {
                    "Unfavorited.".into()
                }
            }
            Err(e) => format!("Favorite failed: {e:#}"),
        };
    }

    fn copy_current_info(&mut self) {
        let Some(song) = self.info.as_ref().and_then(|i| i.current_song()) else {
            self.status = "No song playing.".into();
            return;
        };
        let text = format!(
            "{} — {} ({})",
            song.artists_text(),
            song.title,
            song.album_name()
        );
        self.status = match copy_to_clipboard(&text) {
            Ok(()) => format!("Copied: {text}"),
            Err(e) => format!("{e:#}"),
        };
    }

    fn exec_search(&mut self) {
        let query = self.input_buf.trim().to_string();
        self.input_buf.clear();
        self.input_mode = None;
        if query.is_empty() {
            self.status = "Search cancelled.".into();
            return;
        }
        match self.client.search(self.sid, &query) {
            Ok(search) => {
                self.search = search;
                self.section = Section::Search;
                self.selected = 0;
                self.status = format!("Search: {} results.", self.search.songs.len());
            }
            Err(e) => self.status = format!("Search failed: {e:#}"),
        }
    }

    fn exec_album(&mut self) {
        let id_str = self.input_buf.trim().to_string();
        self.input_buf.clear();
        self.input_mode = None;
        let Ok(id) = id_str.parse::<u64>() else {
            self.status = "Album ID must be a number.".into();
            return;
        };
        match self.client.album(self.sid, id) {
            Ok(album) => {
                self.albums = album.album.into_iter().collect();
                self.search.songs = album.songs;
                self.section = Section::Albums;
                self.status = "Album loaded.".into();
            }
            Err(e) => self.status = format!("Album failed: {e:#}"),
        }
    }

    fn exec_login_user(&mut self) {
        self.login_user = self.input_buf.trim().to_string();
        self.input_buf.clear();
        self.input_cursor = 0;
        self.input_mode = Some(InputMode::LoginKey);
        self.status = "Enter your Rainwave API key.".into();
    }

    fn exec_login_key(&mut self) {
        let api_key = self.input_buf.trim().to_string();
        self.input_buf.clear();
        self.input_mode = None;
        let creds = rainwave_tui::Credentials {
            user_id: self.login_user.clone(),
            api_key,
        };
        match SecretStore::save(&creds) {
            Ok(()) => match RainwaveClient::new(Some(creds)) {
                Ok(client) => {
                    self.client = client;
                    self.logged_in = true;
                    self.status = "Login saved to OS keyring.".into();
                }
                Err(e) => self.status = format!("Client error: {e:#}"),
            },
            Err(e) => self.status = format!("Keyring error: {e:#}"),
        }
        self.login_user.clear();
    }

    fn volume_up(&mut self) {
        let vol = (self.player.volume() + 5).min(150);
        self.player.set_volume(vol);
        if self.player.is_playing() {
            if let Err(e) = self.player.restart(self.current_station()) {
                self.status = format!("Volume restart failed: {e:#}");
                return;
            }
        }
        self.config.volume = vol;
        let _ = self.config.save();
        self.status = format!("Volume: {}%", vol);
    }

    fn volume_down(&mut self) {
        let vol = self.player.volume().saturating_sub(5);
        self.player.set_volume(vol);
        if self.player.is_playing() {
            if let Err(e) = self.player.restart(self.current_station()) {
                self.status = format!("Volume restart failed: {e:#}");
                return;
            }
        }
        self.config.volume = vol;
        let _ = self.config.save();
        self.status = format!("Volume: {}%", vol);
    }

    fn session_uptime(&self) -> String {
        let secs = self.session_start.elapsed().as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        if h > 0 {
            format!("{h}h {m:02}m")
        } else {
            format!("{m}m {s:02}s")
        }
    }
}

// ── Entry point ───────────────────────────────────────────────

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("rainwave-tui {VERSION}");
        println!("A terminal client for Rainwave.cc game music radio");
        return Ok(());
    }

    let mut app = App::new()?;
    app.refresh();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

// ── Main loop ─────────────────────────────────────────────────

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        let interval = Duration::from_secs(app.refresh_interval);
        if app.input_mode.is_none() && !app.show_options && app.last_refresh.elapsed() > interval {
            app.refresh();
        }
        if app.player.is_playing() {
            app.activity = rand::thread_rng().gen_range(0.2..1.0);
        } else if app.activity > 0.0 {
            app.activity = (app.activity - 0.04).max(0.0);
        }
        let size = terminal.size()?;
        app.term_w = size.width as f64;
        app.term_h = size.height as f64;
        app.tick_rain();
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(60))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if app.show_options {
                handle_options_key(app, key.code, key.modifiers);
                continue;
            }
            if let Some(mode) = app.input_mode {
                handle_input_mode(app, mode, key.code);
                continue;
            }
            if app.show_help && key.code != KeyCode::Char('?') && key.code != KeyCode::Esc {
                continue;
            }
            if app.show_detail && key.code != KeyCode::Char('?') && key.code != KeyCode::Esc {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('i') => {
                        app.show_detail = false;
                        continue;
                    }
                    _ => continue,
                }
            }
            let k = &app.keys;
            match key.code {
                KeyCode::Char(c) if c == k.quit => break,
                KeyCode::Char('?') => app.show_help = !app.show_help,
                KeyCode::Esc => {
                    app.show_help = false;
                    app.show_detail = false;
                    app.request_search = false;
                }
                KeyCode::Char(' ') => match app.player.toggle(app.current_station()) {
                    Ok(()) => app.status = "Playback toggled.".into(),
                    Err(e) => app.status = format!("mpv: {e:#}"),
                },
                KeyCode::Char(c) if c == k.rain_toggle => app.rain_show = !app.rain_show,
                KeyCode::Char('R') => app.refresh(),
                KeyCode::Tab => {
                    app.request_search = false;
                    app.section = app.section.next();
                    app.selected = 0;
                }
                KeyCode::BackTab => {
                    app.request_search = false;
                    app.section = app.section.prev();
                    app.selected = 0;
                }
                KeyCode::Down => {
                    app.selected = (app.selected + 1).min(app.active_items_len().saturating_sub(1))
                }
                KeyCode::Up => app.selected = app.selected.saturating_sub(1),
                KeyCode::Char(c) if c == k.down => {
                    app.selected = (app.selected + 1).min(app.active_items_len().saturating_sub(1))
                }
                KeyCode::Char(c) if c == k.up => {
                    app.selected = app.selected.saturating_sub(1)
                }
                KeyCode::Enter => match app.section {
                    Section::Voting => app.vote(),
                    Section::Stations if app.selected < 6 => {
                        app.switch_station(app.selected as u8 + 1)
                    }
                    _ => {}
                },
                KeyCode::Char(c @ '1'..='6') => app.switch_station(c as u8 - b'0'),
                KeyCode::Char(c) if c == k.vote => app.vote(),
                KeyCode::Char(c) if c == k.request => app.request_selected(),
                KeyCode::Char(c) if c == k.delete => {
                    if let Some(id) = app.selected_song_id() {
                        app.status = app
                            .client
                            .delete_request(app.sid, id)
                            .map(|_| "Request removed.".into())
                            .unwrap_or_else(|e| format!("Delete failed: {e:#}"));
                    }
                }
                KeyCode::Char(c) if c == k.clear => {
                    app.status = app
                        .client
                        .clear_requests(app.sid)
                        .map(|_| "Request queue cleared.".into())
                        .unwrap_or_else(|e| format!("Clear failed: {e:#}"))
                }
                KeyCode::Char(c) if c == k.fave => app.fave_current(),
                KeyCode::Char(c @ '0'..='5') => app.rate_current((c as u8 - b'0') as f64),
                KeyCode::Char(c) if c == k.search || c == '/' => {
                    if app.section == Section::Requests {
                        app.request_search = true;
                    }
                    app.input_mode = Some(InputMode::Search);
                    app.input_buf.clear();
                    app.input_cursor = 0;
                    app.status = "Type search query, Enter to confirm, Esc to cancel.".into();
                }
                KeyCode::Char(c) if c == k.album => {
                    app.input_mode = Some(InputMode::Album);
                    app.input_buf.clear();
                    app.input_cursor = 0;
                    app.status = "Type album ID, Enter to confirm, Esc to cancel.".into();
                }
                KeyCode::Char(c) if c == k.login => {
                    app.input_mode = Some(InputMode::LoginUser);
                    app.input_buf.clear();
                    app.input_cursor = 0;
                    app.status = "Enter your Rainwave user ID.".into();
                }
                KeyCode::Char(c) if c == k.logout => {
                    SecretStore::clear()?;
                    app.client = RainwaveClient::new(None)?;
                    app.logged_in = false;
                    app.status = "Logged out and keyring secrets cleared.".into();
                }
                KeyCode::Char(c) if c == k.history => {
                    app.section = Section::History;
                    app.selected = 0;
                }
                KeyCode::Char(c) if c == k.copy_info => app.copy_current_info(),
                KeyCode::Char(c) if c == k.detail => {
                    app.show_detail = !app.show_detail;
                }
                KeyCode::F(1) | KeyCode::Char('O') => {
                    app.show_options = true;
                    app.options_tab = OptionsTab::Palette;
                    app.options_selected = 0;
                    app.status = "Options — Esc to close.".into();
                }
                KeyCode::Char('+') | KeyCode::Char('=') => app.volume_up(),
                KeyCode::Char('-') => {
                    if app.section == Section::Requests {
                        if let Some(id) = app.selected_song_id() {
                            app.status = app
                                .client
                                .delete_request(app.sid, id)
                                .map(|_| "Request removed.".into())
                                .unwrap_or_else(|e| format!("Delete failed: {e:#}"));
                        }
                    } else {
                        app.volume_down();
                    }
                }
                _ => {}
            }
        }
    }
    app.save_config();
    Ok(())
}

// ── Options key handler ───────────────────────────────────────

fn handle_options_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) {
    match code {
        KeyCode::Esc => {
            app.show_options = false;
            app.status = "Options closed.".into();
        }
        KeyCode::Tab => {
            app.options_tab = match app.options_tab {
                OptionsTab::Palette => OptionsTab::General,
                OptionsTab::General => OptionsTab::Layout,
                OptionsTab::Layout => OptionsTab::Palette,
            };
            app.options_selected = 0;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = match app.options_tab {
                OptionsTab::Palette => 0,
                OptionsTab::General => 3,
                OptionsTab::Layout => KeyLayout::ALL.len().saturating_sub(1),
            };
            app.options_selected = (app.options_selected + 1).min(max);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.options_selected = app.options_selected.saturating_sub(1);
        }
        KeyCode::Left => match app.options_tab {
            OptionsTab::Palette => {
                let n = THEMES.len();
                app.theme_idx = (app.theme_idx + n - 1) % n;
                app.status = format!("Theme: {}", THEMES[app.theme_idx].name);
            }
            OptionsTab::General => match app.options_selected {
                0 => {
                    app.refresh_interval = app.refresh_interval.saturating_sub(5).max(5);
                    app.status = format!("Refresh: {}s", app.refresh_interval);
                }
                2 => {
                    app.config.notifications = !app.config.notifications;
                    app.status = format!(
                        "Notifications: {}",
                        if app.config.notifications {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                }
                _ => {}
            },
            OptionsTab::Layout => {
                let current = app.config.layout;
                app.config.layout = match current {
                    KeyLayout::Qwerty => KeyLayout::Colemak,
                    KeyLayout::Dvorak => KeyLayout::Qwerty,
                    KeyLayout::Colemak => KeyLayout::Dvorak,
                };
                app.keys = KeyBindings::for_layout(app.config.layout);
                app.status = format!("Layout: {}", app.config.layout.name());
            }
        },
        KeyCode::Right => match app.options_tab {
            OptionsTab::Palette => {
                app.theme_idx = (app.theme_idx + 1) % THEMES.len();
                app.status = format!("Theme: {}", THEMES[app.theme_idx].name);
            }
            OptionsTab::General => match app.options_selected {
                0 => {
                    app.refresh_interval = (app.refresh_interval + 5).min(120);
                    app.status = format!("Refresh: {}s", app.refresh_interval);
                }
                2 => {
                    app.config.notifications = !app.config.notifications;
                    app.status = format!(
                        "Notifications: {}",
                        if app.config.notifications {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                }
                _ => {}
            },
            OptionsTab::Layout => {
                app.config.layout = app.config.layout.next();
                app.keys = KeyBindings::for_layout(app.config.layout);
                app.status = format!("Layout: {}", app.config.layout.name());
            }
        },
        KeyCode::Char('r') => app.rain_show = !app.rain_show,
        _ => {}
    }
}

// ── Input mode handler ────────────────────────────────────────

fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len())
}

fn handle_input_mode(app: &mut App, mode: InputMode, code: KeyCode) {
    match code {
        KeyCode::Enter => match mode {
            InputMode::Search => app.exec_search(),
            InputMode::Album => app.exec_album(),
            InputMode::LoginUser => app.exec_login_user(),
            InputMode::LoginKey => app.exec_login_key(),
        },
        KeyCode::Esc => {
            app.input_mode = None;
            app.request_search = false;
            app.input_buf.clear();
            app.input_cursor = 0;
            app.login_user.clear();
            app.status = "Cancelled.".into();
        }
        KeyCode::Char(c) => {
            let byte_idx = char_to_byte_index(&app.input_buf, app.input_cursor);
            app.input_buf.insert(byte_idx, c);
            app.input_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
                let byte_idx = char_to_byte_index(&app.input_buf, app.input_cursor);
                app.input_buf.remove(byte_idx);
            }
        }
        KeyCode::Delete => {
            let char_count = app.input_buf.chars().count();
            if app.input_cursor < char_count {
                let byte_idx = char_to_byte_index(&app.input_buf, app.input_cursor);
                app.input_buf.remove(byte_idx);
            }
        }
        KeyCode::Left => {
            app.input_cursor = app.input_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            let char_count = app.input_buf.chars().count();
            app.input_cursor = char_count.min(app.input_cursor + 1);
        }
        KeyCode::Home => app.input_cursor = 0,
        KeyCode::End => app.input_cursor = app.input_buf.chars().count(),
        _ => {}
    }
}

// ── Top-level draw ────────────────────────────────────────────

fn draw(f: &mut ratatui::Frame<'_>, app: &mut App) {
    if app.rain_show {
        draw_rain(f, app);
    }
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(f.area());
    draw_now_playing(f, root[0], app);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32),
            Constraint::Min(36),
            Constraint::Length(34),
        ])
        .split(root[1]);
    draw_stations(f, cols[0], app);
    if app.section == Section::History {
        draw_history(f, cols[1], app);
    } else {
        draw_center(f, cols[1], app);
    }
    draw_side(f, cols[2], app);
    draw_status_bar(f, root[2], app);
    if app.show_help {
        draw_help(f, app);
    }
    if app.show_options {
        draw_options(f, app);
    }
    if app.show_detail {
        draw_detail(f, app);
    }
    if app.input_mode.is_some() {
        draw_input_overlay(f, app);
    }
}

// ── Rain background ───────────────────────────────────────────

fn draw_rain(f: &mut ratatui::Frame<'_>, app: &App) {
    let t = theme(app);
    let (w, h) = (app.term_w as u16, app.term_h as u16);
    for drop in &app.raindrops {
        let x = drop.x.round() as u16;
        let y = drop.y.round() as u16;
        if x >= w || y >= h {
            continue;
        }
        let b = drop.bright.clamp(0.0, 1.0);
        let col = mix_color(t.dim, t.accent, b);
        let area = Rect {
            x,
            y,
            width: 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                drop.ch.to_string(),
                Style::new().fg(col).bg(Color::Reset),
            ))),
            area,
        );
    }
}

fn mix_color(a: Color, b: Color, t: f64) -> Color {
    fn to_rgb(c: Color) -> (u8, u8, u8) {
        match c {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::DarkGray => (80, 80, 80),
            Color::LightCyan => (224, 255, 255),
            Color::LightMagenta => (255, 224, 255),
            Color::LightYellow => (255, 255, 224),
            Color::LightGreen => (144, 238, 144),
            Color::LightBlue => (173, 216, 230),
            Color::Cyan => (0, 255, 255),
            Color::Magenta => (255, 0, 255),
            Color::Yellow => (255, 255, 0),
            Color::Green => (0, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::White => (255, 255, 255),
            Color::Black => (0, 0, 0),
            Color::Red => (255, 0, 0),
            _ => (128, 128, 128),
        }
    }
    let (r1, g1, b1) = to_rgb(a);
    let (r2, g2, b2) = to_rgb(b);
    let lerp = |a: f64, b: f64, t: f64| (a + (b - a) * t).round() as u8;
    Color::Rgb(
        lerp(r1 as f64, r2 as f64, t),
        lerp(g1 as f64, g2 as f64, t),
        lerp(b1 as f64, b2 as f64, t),
    )
}

// ── Status bar ────────────────────────────────────────────────

fn draw_status_bar(f: &mut ratatui::Frame<'_>, area: Rect, app: &mut App) {
    let t = theme(app);
    let rain_sym = if app.rain_show { "●" } else { "○" };
    let play_sym = if app.player.is_playing() {
        "▶"
    } else {
        "■"
    };
    let vol = app.player.volume();
    let vol_bar = format!("{vol}%");
    let hints = format!(
        "{} | {play_sym} {} | Vol: {vol_bar} | {rain_sym} rain | R: refresh",
        app.section_hints(),
        app.current_station().name
    );
    let uptime = app.session_uptime();
    let bottom_title = format!(
        " {} | {} songs | {} | {} ",
        app.section.title(),
        app.songs_heard,
        uptime,
        app.status
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, Style::new().fg(t.dim))))
            .block(Block::default().borders(Borders::ALL).title(bottom_title)),
        area,
    );
}

// ── Now Playing panel ─────────────────────────────────────────

fn draw_now_playing(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let t = theme(app);
    let song = app.info.as_ref().and_then(|i| i.current_song());
    let title = song
        .map(|s| s.title.as_str())
        .unwrap_or("Loading Rainwave...");
    let artist = song.map(|s| s.artists_text()).unwrap_or_default();
    let album_name = song.map(|s| s.album_name()).unwrap_or_default();
    let ratio = app.info.as_ref().map(|i| i.progress()).unwrap_or(0.0);
    let next_songs: Vec<String> = app
        .info
        .as_ref()
        .map(|i| {
            i.sched_next
                .first()
                .map(|s| {
                    s.songs
                        .iter()
                        .take(3)
                        .map(|s2| s2.title.clone())
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let rows = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(31), Constraint::Min(24)])
        .split(area);

    let art_lines = if app.art.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No art yet",
                Style::new().fg(t.dim),
            )),
            Line::from(""),
        ]
    } else {
        art_lines(&app.art)
    };
    f.render_widget(
        Paragraph::new(art_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Album Art ")
                .border_style(t.border_art),
        ),
        rows[0],
    );

    let song_len = song.and_then(|s| s.length).unwrap_or(0);
    let elapsed = app
        .info
        .as_ref()
        .map(|i| i.elapsed_secs().max(0) as u64)
        .unwrap_or(0);
    let remaining = app
        .info
        .as_ref()
        .map(|i| i.remaining_secs().max(0) as u64)
        .unwrap_or(0);
    let time_str = if song_len > 0 {
        format!(
            "{:02}:{:02} / {:02}:{:02}  ({}:{:02} left)",
            elapsed / 60,
            elapsed % 60,
            song_len / 60,
            song_len % 60,
            remaining / 60,
            remaining % 60
        )
    } else {
        String::new()
    };

    let fave_mark = song
        .and_then(|s| s.fave)
        .map(|f| if f { " ♥" } else { "" })
        .unwrap_or("");

    let bar_w = (ratio * 24.0).round() as usize;
    let filled = "█".repeat(bar_w.min(24));
    let empty = "░".repeat(24usize.saturating_sub(bar_w));
    let bar_spans = vec![
        Span::styled(filled, Style::new().fg(t.gauge_fg)),
        Span::styled(
            empty,
            Style::new().fg(t.gauge_bg),
        ),
        Span::styled(
            format!(" {:>3}%", (ratio * 100.0) as u8),
            Style::new().fg(t.dim),
        ),
    ];

    let (stars_text, stars_color) = song
        .and_then(|s| s.rating)
        .map(rainwave_tui::rating_stars)
        .unwrap_or_else(|| ("☆☆☆☆☆".into(), t.dim));

    let station_info = app
        .info
        .as_ref()
        .and_then(|i| i.all_stations_info.get(&app.sid.to_string()));

    let mut info_lines = vec![
        Line::from(vec![
            Span::styled(
                "♫ ",
                Style::new().fg(t.now_playing).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                title,
                Style::new()
                    .fg(t.now_playing)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(fave_mark, Style::new().fg(t.error)),
        ]),
        Line::from(vec![
            Span::styled("by ", Style::new().fg(t.dim)),
            Span::styled(&artist, Style::new().fg(t.artist)),
        ]),
        Line::from(vec![
            Span::styled("on ", Style::new().fg(t.dim)),
            Span::styled(&album_name, Style::new().fg(t.album)),
        ]),
        Line::from(vec![
            Span::styled(&stars_text, Style::new().fg(stars_color)),
            Span::styled(
                format!("  {}/5", song.map(|s| s.rating_text()).unwrap_or_default()),
                Style::new().fg(t.dim),
            ),
        ]),
        Line::from(""),
    ];
    if !time_str.is_empty() {
        info_lines.push(Line::from(Span::styled(&time_str, Style::new().fg(t.dim))));
    }
    if !next_songs.is_empty() {
        info_lines.push(Line::from(vec![
            Span::styled("Next: ", Style::new().fg(t.dim)),
            Span::raw(next_songs.join("  │  ")),
        ]));
    }
    info_lines.push(Line::from(bar_spans));

    let border_title = if let Some(ns) = station_info {
        if !ns.event_type.is_empty() {
            format!(" {} — {} ", app.current_station().name, ns.event_type)
        } else {
            format!(" Rainwave {} ", app.current_station().name)
        }
    } else {
        format!(" Rainwave {} ", app.current_station().name)
    };

    f.render_widget(
        Paragraph::new(info_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(border_title)
                    .border_style(t.border_primary),
            )
            .wrap(Wrap { trim: true }),
        rows[1],
    );
}

// ── Stations panel ────────────────────────────────────────────

fn draw_stations(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let t = theme(app);
    let items: Vec<ListItem> = STATIONS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let marker = if s.id == app.sid { "▶" } else { " " };
            let now = app
                .info
                .as_ref()
                .and_then(|info| info.all_stations_info.get(&s.id.to_string()))
                .map(|n| {
                    let title = if n.title.len() > 16 {
                        format!("{}…", &n.title[..15])
                    } else {
                        n.title.clone()
                    };
                    format!(" {title}")
                })
                .unwrap_or_default();
            let style = if app.section == Section::Stations && app.selected == i {
                Style::new()
                    .fg(t.highlight_fg)
                    .bg(t.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else if s.id == app.sid {
                Style::new().fg(t.accent)
            } else {
                Style::new().fg(Color::White)
            };
            let num = format!("{:>2}", s.id);
            ListItem::new(format!("{marker} {} {:<12}{}", num, s.name, now)).style(style)
        })
        .collect();
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Stations [{}] ",
                    app.config.layout.name()
                ))
                .border_style(t.border_secondary),
        ),
        area,
    );
}

// ── Center panels ─────────────────────────────────────────────

fn draw_center(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let t = theme(app);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let election = app
        .info
        .as_ref()
        .and_then(|i| i.sched_next.first())
        .map(|e| e.songs.as_slice())
        .unwrap_or(&[]);
    let vote_items = if election.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No election in progress",
            Style::new().fg(t.dim),
        )))]
    } else {
        election
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let votes = s.entry_votes.unwrap_or(0);
                let (stars, _sc) = s
                    .rating
                    .map(rainwave_tui::rating_stars)
                    .unwrap_or_else(|| ("☆☆☆☆☆".into(), t.dim));
                themed_item(
                    t,
                    i,
                    app.section == Section::Voting && app.selected == i,
                    format!(
                        "{votes} votes  {} — {}  {stars}",
                        s.title,
                        s.album_name()
                    ),
                )
            })
            .collect::<Vec<_>>()
    };
    f.render_widget(
        List::new(vote_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Voting ")
                .border_style(t.border_vote),
        ),
        rows[0],
    );
    let search_items = if app.search.songs.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  Press / to search the catalog",
            Style::new().fg(t.dim),
        )))]
    } else {
        app.search
            .songs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (stars, _sc) = s
                    .rating
                    .map(rainwave_tui::rating_stars)
                    .unwrap_or_else(|| ("☆☆☆☆☆".into(), t.dim));
                themed_item(
                    t,
                    i,
                    app.section == Section::Search && app.selected == i,
                    format!("{} — {}  {stars}", s.title, s.album_name()),
                )
            })
            .collect::<Vec<_>>()
    };
    f.render_widget(
        List::new(search_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Search Results [{}]", app.search.songs.len()))
                .border_style(t.border_search),
        ),
        rows[1],
    );
}

// ── Side panels ───────────────────────────────────────────────

fn draw_side(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let t = theme(app);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(35),
            Constraint::Percentage(20),
        ])
        .split(area);
    if app.request_search {
        let req_items = if app.search.songs.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No results",
                Style::new().fg(t.dim),
            )))]
        } else {
            app.search
                .songs
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let (stars, _) = s
                        .rating
                        .map(rainwave_tui::rating_stars)
                        .unwrap_or_else(|| ("☆".into(), t.dim));
                    themed_item(
                        t,
                        i,
                        app.section == Section::Requests && app.selected == i,
                        format!(
                            "{:<36} {:>8}  {stars}",
                            truncate(&s.title, 36),
                            s.length_text()
                        ),
                    )
                })
                .collect::<Vec<_>>()
        };
        f.render_widget(
            List::new(req_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search to Request ")
                    .border_style(t.border_req),
            ),
            rows[0],
        );
    } else {
        let reqs = app
            .info
            .as_ref()
            .map(|i| i.request_line.as_slice())
            .unwrap_or(&[]);
        let req_items = if reqs.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No requests in queue",
                Style::new().fg(t.dim),
            )))]
        } else {
            reqs.iter()
                .enumerate()
                .map(|(i, r)| {
                    themed_item(
                        t,
                        i,
                        app.section == Section::Requests && app.selected == i,
                        format!(
                            "#{} {}: {}",
                            r.position,
                            r.username,
                            r.song
                                .as_ref()
                                .map(|s| s.title.as_str())
                                .unwrap_or("waiting")
                        ),
                    )
                })
                .collect::<Vec<_>>()
        };
        f.render_widget(
            List::new(req_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Requests [{}]", reqs.len()))
                    .border_style(t.border_req),
            ),
            rows[0],
        );
    }
    let album_items = if app.albums.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  Press a to look up album by ID",
            Style::new().fg(t.dim),
        )))]
    } else {
        app.albums
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let (stars, _sc) = a
                    .rating
                    .map(rainwave_tui::rating_stars)
                    .unwrap_or_else(|| ("☆☆☆☆☆".into(), t.dim));
                themed_item(
                    t,
                    i,
                    app.section == Section::Albums && app.selected == i,
                    format!("{}  {stars}", truncate(&a.name, 28)),
                )
            })
            .collect::<Vec<_>>()
    };
    f.render_widget(
        List::new(album_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Albums [{}]", app.albums.len()))
                .border_style(t.border_album),
        ),
        rows[1],
    );
    let login = if app.logged_in {
        vec![
            Line::from(Span::styled(
                "● Logged in",
                Style::new().fg(t.success).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  Secrets stored in OS keyring",
                Style::new().fg(t.dim),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "○ Anonymous mode",
                Style::new().fg(t.dim),
            )),
            Line::from(Span::styled(
                format!("  Press {} to store credentials", app.keys.login),
                Style::new().fg(t.dim),
            )),
        ]
    };
    f.render_widget(
        Paragraph::new(login).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Login ")
                .border_style(t.border_login),
        ),
        rows[2],
    );
}

// ── History panel ─────────────────────────────────────────────

fn draw_history(f: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let t = theme(app);
    let items = if app.history.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No songs heard this session",
            Style::new().fg(t.dim),
        )))]
    } else {
        app.history
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let (stars, _) = h
                    .rating
                    .map(rainwave_tui::rating_stars)
                    .unwrap_or_else(|| ("☆".into(), t.dim));
                let style = if app.section == Section::History && app.selected == i {
                    Style::new()
                        .fg(t.highlight_fg)
                        .bg(t.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::new()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>3}. ", i + 1),
                        Style::new().fg(t.dim),
                    ),
                    Span::styled(&h.artist, Style::new().fg(t.artist)),
                    Span::styled(" — ", Style::new().fg(t.dim)),
                    Span::styled(&h.title, Style::new().fg(Color::White)),
                    Span::styled(format!("  {stars}"), Style::new().fg(t.dim)),
                ]))
                .style(style)
            })
            .collect::<Vec<_>>()
    };
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" History [{}] ", app.history.len()))
                .border_style(t.border_history),
        ),
        area,
    );
}

// ── Detail overlay (game/artist info) ─────────────────────────

fn draw_detail(f: &mut ratatui::Frame<'_>, app: &App) {
    let t = theme(app);
    let w = 56u16;
    let h = 18u16;
    let area = centered_rect(w, h, f.area());
    f.render_widget(Clear, area);

    let song = app.info.as_ref().and_then(|i| i.current_song());
    let mut lines = vec![
        Line::from(Span::styled(
            " Song Information ",
            Style::new()
                .fg(t.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        Line::from(""),
    ];

    if let Some(song) = song {
        let (stars, sc) = song
            .rating
            .map(rainwave_tui::rating_stars)
            .unwrap_or_else(|| ("☆☆☆☆☆".into(), t.dim));
        let user_rating = song.rating_user.map(|r| format!("{r:.1}")).unwrap_or("-".into());
        let fave = song.fave.map(|f| if f { "♥ yes" } else { "○ no" }).unwrap_or("-".into());
        let len = song.length_text();
        let votes = song.entry_votes.map(|v| v.to_string()).unwrap_or("-".into());
        let rating_text = song.rating_text();
        let artists = song.artists_text();
        let album = song.album_name();
        let title = song.title.clone();
        let fave_val = song.fave.unwrap_or(false);
        let user_rating = user_rating;
        let album_rating = song.albums.first().and_then(|a| a.rating).map(rainwave_tui::rating_stars);

        lines.extend([
            Line::from(vec![
                Span::styled("  Title:   ", Style::new().fg(t.dim)),
                Span::styled(title, Style::new().fg(t.now_playing).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Artist:  ", Style::new().fg(t.dim)),
                Span::styled(artists, Style::new().fg(t.artist)),
            ]),
            Line::from(vec![
                Span::styled("  Album:   ", Style::new().fg(t.dim)),
                Span::styled(album, Style::new().fg(t.album)),
            ]),
            Line::from(vec![
                Span::styled("  Rating:  ", Style::new().fg(t.dim)),
                Span::styled(stars, Style::new().fg(sc)),
                Span::styled(
                    format!(" ({rating_text})  Your: {user_rating}"),
                    Style::new().fg(t.dim),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Length:  ", Style::new().fg(t.dim)),
                Span::styled(len, Style::new().fg(Color::White)),
                Span::styled(
                    format!("   Votes: {votes}"),
                    Style::new().fg(t.dim),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Faved:   ", Style::new().fg(t.dim)),
                Span::styled(
                    fave,
                    Style::new().fg(if fave_val { t.error } else { t.dim }),
                ),
            ]),
            Line::from(""),
        ]);

        if let Some(album) = song.albums.first() {
            lines.extend([
                Line::from(Span::styled(
                    " Album Info ",
                    Style::new()
                        .fg(t.album)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )),
                Line::from(vec![
                    Span::styled("  ID:      ", Style::new().fg(t.dim)),
                    Span::styled(album.id.to_string(), Style::new().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  Name:    ", Style::new().fg(t.dim)),
                    Span::styled(album.name.clone(), Style::new().fg(t.album)),
                ]),
            ]);
            if let Some((astars, asc)) = album_rating {
                lines.push(Line::from(vec![
                    Span::styled("  Rating:  ", Style::new().fg(t.dim)),
                    Span::styled(astars, Style::new().fg(asc)),
                ]));
            }
        }

        if !song.artists.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Artists ",
                Style::new()
                    .fg(t.artist)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            for artist in &song.artists {
                let id_str = artist
                    .id
                    .map(|id| format!(" (id: {id})"))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::new().fg(t.accent)),
                    Span::styled(artist.name.clone(), Style::new().fg(t.artist)),
                    Span::styled(id_str, Style::new().fg(t.dim)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No song information available",
            Style::new().fg(t.dim),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc/i to close  |  y to copy info",
        Style::new().fg(t.dim),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ♫ Song & Artist Detail ")
        .border_style(t.border_info);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

// ── Options overlay ───────────────────────────────────────────

fn draw_options(f: &mut ratatui::Frame<'_>, app: &App) {
    let t = theme(app);
    let w = 66u16;
    let h = 20u16;
    let area = centered_rect(w, h, f.area());
    f.render_widget(Clear, area);

    let tab_names = ["  Palette  ", "  General  ", "  Layout   "];
    let tabs: Vec<Span> = tab_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_active = (i == 0 && app.options_tab == OptionsTab::Palette)
                || (i == 1 && app.options_tab == OptionsTab::General)
                || (i == 2 && app.options_tab == OptionsTab::Layout);
            if is_active {
                Span::styled(
                    *name,
                    Style::new()
                        .fg(t.highlight_fg)
                        .bg(t.highlight_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(*name, Style::new().fg(t.dim))
            }
        })
        .collect();

    let mut lines = vec![Line::from(tabs), Line::from("")];

    match app.options_tab {
        OptionsTab::Palette => {
            lines.push(Line::from(Span::styled(
                format!("Current: {}", THEMES[app.theme_idx].name),
                Style::new().fg(t.accent).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "← →  Cycle themes  |  r  Toggle rain effect",
                Style::new().fg(t.dim),
            )));
            lines.push(Line::from(""));
            let swatch = THEMES
                .iter()
                .enumerate()
                .flat_map(|(i, th)| {
                    let sel = if i == app.theme_idx { "●" } else { "○" };
                    let spans = vec![
                        Span::styled(sel, Style::new().fg(th.accent)),
                        Span::styled(
                            format!(" {}", th.name),
                            Style::new().fg(if i == app.theme_idx {
                                th.accent
                            } else {
                                t.dim
                            }),
                        ),
                        Span::raw("  "),
                    ];
                    spans
                })
                .collect::<Vec<_>>();
            lines.push(Line::from(swatch));
            lines.push(Line::from(""));
            let rain_indicator = if app.rain_show {
                Span::styled("● Rain: ON ", Style::new().fg(t.accent))
            } else {
                Span::styled("○ Rain: OFF", Style::new().fg(t.dim))
            };
            lines.push(Line::from(rain_indicator));
        }
        OptionsTab::General => {
            let sel0 = app.options_selected == 0;
            let sel1 = app.options_selected == 1;
            let sel2 = app.options_selected == 2;
            lines.push(Line::from(Span::styled(
                format!(
                    "{}Refresh interval:  {}s",
                    if sel0 { "▸ " } else { "  " },
                    app.refresh_interval
                ),
                if sel0 { selt(t) } else { Style::new().fg(Color::White) },
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "{}MPV binary:       {}",
                    if sel1 { "▸ " } else { "  " },
                    app.mpv_binary
                ),
                if sel1 { selt(t) } else { Style::new().fg(Color::White) },
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "{}Notifications:    {}",
                    if sel2 { "▸ " } else { "  " },
                    if app.config.notifications { "ON" } else { "OFF" }
                ),
                if sel2 { selt(t) } else { Style::new().fg(Color::White) },
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "← → to adjust  |  Tab: switch tab  |  Esc: close",
                Style::new().fg(t.dim),
            )));
        }
        OptionsTab::Layout => {
            lines.push(Line::from(Span::styled(
                "Keyboard Layout",
                Style::new()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Same physical keys, different characters",
                Style::new().fg(t.dim),
            )));
            lines.push(Line::from(""));
            for (i, layout) in KeyLayout::ALL.iter().enumerate() {
                let is_current = *layout == app.config.layout;
                let is_sel = i == app.options_selected;
                let marker = if is_current { "●" } else { "○" };
                let prefix = if is_sel { "▸ " } else { "  " };
                let name = layout.name();
                let style = if is_current {
                    selt(t)
                } else if is_sel {
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(t.dim)
                };
                lines.push(Line::from(Span::styled(
                    format!("{prefix}{marker} {name}{}", if is_current { "  (active)" } else { "" }),
                    style,
                )));
            }
            lines.push(Line::from(""));
            let kb = &app.keys;
            lines.push(Line::from(Span::styled(
                " Key Mapping",
                Style::new()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "  quit:{} vote:{} request:{} delete:{}",
                    kb.quit, kb.vote, kb.request, kb.delete
                ),
                Style::new().fg(t.dim),
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "  up:{} down:{} fave:{} search:{}",
                    kb.up, kb.down, kb.fave, kb.search
                ),
                Style::new().fg(t.dim),
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "  login:{} history:{} info:{} copy:{}",
                    kb.login, kb.history, kb.detail, kb.copy_info
                ),
                Style::new().fg(t.dim),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "← → to select  |  Tab: switch tab  |  Esc: close",
                Style::new().fg(t.dim),
            )));
        }
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Options — F1 {} ", app.config.layout.name()))
                .border_style(t.border_secondary),
        ),
        area,
    );
}

// ── Input overlay ─────────────────────────────────────────────

fn draw_input_overlay(f: &mut ratatui::Frame<'_>, app: &App) {
    let t = theme(app);
    let label = match app.input_mode.unwrap() {
        InputMode::Search => "Search Rainwave",
        InputMode::Album => "Album ID",
        InputMode::LoginUser => "Rainwave User ID",
        InputMode::LoginKey => "API Key (visible in TUI)",
    };
    let display = if app.input_mode == Some(InputMode::LoginKey) {
        "•".repeat(app.input_buf.chars().count())
    } else {
        app.input_buf.clone()
    };
    let char_count = display.chars().count();
    let cursor_show = if app.input_cursor < char_count {
        let before: String = display.chars().take(app.input_cursor).collect();
        let at = display.chars().nth(app.input_cursor).unwrap_or(' ');
        format!("{before}{at}")
    } else {
        format!("{display} ")
    };

    let lines = vec![
        Line::from(Span::styled(
            label,
            Style::new().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::raw(format!("> {cursor_show}"))),
        Line::from(""),
        Line::from(Span::styled(
            "Enter  confirm  |  Esc  cancel  |  ← →  move cursor",
            Style::new().fg(t.dim),
        )),
    ];
    let h = 7;
    let w = 50;
    let area = centered_rect(w, h, f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Input ")
                .border_style(t.border_secondary),
        ),
        area,
    );
}

// ── Help overlay ──────────────────────────────────────────────

fn draw_help(f: &mut ratatui::Frame<'_>, app: &App) {
    let t = theme(app);
    let k = &app.keys;
    let layout_name = app.config.layout.name();
    let help = format!(
        "\
┌─ Navigation ──────────────────────────────────────┐\n\
│  Tab / Shift+Tab     Cycle sections                │\n\
│  {up}/{down} / {up_k}/{down_k}              Move selection               │\n\
│  Enter               Activate selected              │\n\
│  ?                   Toggle this help               │\n\
│  Esc                 Close help / panels            │\n\
├─ Playback ────────────────────────────────────────┤\n\
│  Space               Play / pause                   │\n\
│  1-6                 Switch station                 │\n\
│  R                   Refresh now playing            │\n\
│  +/= / -             Volume up / down               │\n\
├─ Actions ({layout}) ───────────────────────────────┤\n\
│  {vote_k}                  Vote on selected entry         │\n\
│  {req_k}                  Request selected song          │\n\
│  {del_k}                  Delete selected request        │\n\
│  {clr_k}                  Clear request queue            │\n\
│  0-5                 Rate song (0 = clear)          │\n\
│  {fave_k}                  Toggle favorite current song   │\n\
├─ Search & Browse ─────────────────────────────────┤\n\
│  {search_k}                  Search (also from Requests)    │\n\
│  {album_k}                  Browse album by ID             │\n\
├─ Info & Tools ────────────────────────────────────┤\n\
│  {detail_k}                  Song & artist info panel       │\n\
│  {copy_k}                  Copy song info to clipboard    │\n\
│  {hist_k}                  View song history              │\n\
├─ Account ─────────────────────────────────────────┤\n\
│  {login_k}                  Log in (OS keyring)            │\n\
│  {logout_k}                  Log out & clear keyring        │\n\
├─ Options ─────────────────────────────────────────┤\n\
│  O / F1              Open options menu              │\n\
│  {rain_k}                  Toggle rain effect             │\n\
│  q                   Quit                           │\n\
└───────────────────────────────────────────────────┘",
        up = k.up,
        down = k.down,
        up_k = "↑",
        down_k = "↓",
        vote_k = k.vote,
        req_k = k.request,
        del_k = k.delete,
        clr_k = k.clear,
        fave_k = k.fave,
        search_k = k.search,
        album_k = k.album,
        detail_k = k.detail,
        copy_k = k.copy_info,
        hist_k = k.history,
        login_k = k.login,
        logout_k = k.logout,
        rain_k = k.rain_toggle,
        layout = layout_name,
    );
    let lines = help.lines().count() as u16 + 2;
    let area = centered_rect(60, lines, f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(help)
            .style(Style::new().fg(t.help_fg).bg(t.help_bg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Help — {layout_name} layout — ? to close "))
                    .border_style(t.border_secondary),
            ),
        area,
    );
}

// ── Helpers ───────────────────────────────────────────────────

fn selt(t: &Theme) -> Style {
    Style::new()
        .fg(t.highlight_fg)
        .bg(t.highlight_bg)
        .add_modifier(Modifier::BOLD)
}

fn themed_item(t: &Theme, _i: usize, selected: bool, text: String) -> ListItem<'static> {
    let style = if selected {
        selt(t)
    } else {
        Style::new()
    };
    ListItem::new(text).style(style)
}

fn art_lines(art: &[Vec<(u8, u8, u8)>]) -> Vec<Line<'static>> {
    art.iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|&(r, g, b)| {
                        Span::styled(
                            "▀▀",
                            Style::new().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(
                                r / 3,
                                g / 3,
                                b / 3,
                            )),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}…")
    } else {
        s.to_string()
    }
}

