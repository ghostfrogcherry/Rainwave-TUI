use crate::api::RainwaveApi;
use crate::auth::AuthConfig;
use crate::models::*;
use crate::player::Player;
use crate::ui::views::*;
use crate::ui::widgets::styled_block;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::io;
use std::time::{Duration, Instant};

pub enum AppView {
    NowPlaying,
    Stations,
    Requests,
    Search,
    Albums,
    AlbumView(Album),
    Help,
    Login,
}

impl AppView {
    fn label(&self) -> &'static str {
        match self {
            AppView::NowPlaying => "Now Playing",
            AppView::Stations => "Stations",
            AppView::Requests => "Requests",
            AppView::Search => "Search",
            AppView::Albums => "Albums",
            AppView::AlbumView(_) => "Album",
            AppView::Help => "Help",
            AppView::Login => "Login",
        }
    }
}

pub enum InputMode {
    Normal,
    Search,
    Rating,
    LoginUsername,
    LoginPassword,
}

pub struct App {
    pub api: RainwaveApi,
    pub player: Player,
    pub auth: AuthConfig,
    pub current_view: AppView,
    pub input_mode: InputMode,
    pub sync_data: Option<SyncData>,
    pub stations: Vec<Station>,
    pub selected_station: usize,
    pub selected_election: usize,
    pub selected_request: usize,
    pub selected_album: usize,
    pub selected_song: usize,
    pub search_query: String,
    pub search_results: Vec<Song>,
    pub selected_search: usize,
    pub albums: Vec<Album>,
    pub current_album_songs: Vec<Song>,
    pub show_help: bool,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub rating_input: String,
    pub login_username: String,
    pub login_password: String,
    pub animation_tick: u64,
    pub transition_frames: u8,
    pub transition_label: String,
    pub pending_search: Option<String>,
    pub album_art: Option<AlbumArt>,
    pub album_art_url: Option<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let auth = AuthConfig::load();
        
        let api = if auth.is_logged_in() {
            RainwaveApi::with_auth(
                auth.user_id.unwrap(),
                auth.api_key.clone().unwrap(),
                auth.station_id.unwrap_or(1),
            )
        } else {
            RainwaveApi::new()
        };

        Ok(Self {
            api,
            player: Player::new(),
            auth,
            current_view: AppView::NowPlaying,
            input_mode: InputMode::Normal,
            sync_data: None,
            stations: Vec::new(),
            selected_station: 0,
            selected_election: 0,
            selected_request: 0,
            selected_album: 0,
            selected_song: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            selected_search: 0,
            albums: Vec::new(),
            current_album_songs: Vec::new(),
            show_help: false,
            status_message: None,
            should_quit: false,
            rating_input: String::new(),
            login_username: String::new(),
            login_password: String::new(),
            animation_tick: 0,
            transition_frames: 0,
            transition_label: "Now Playing".to_string(),
            pending_search: None,
            album_art: None,
            album_art_url: None,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        self.stations = self.api.get_stations().await.unwrap_or_default();
        self.refresh_data().await?;
        
        if let Some(station_id) = self.auth.station_id {
            self.selected_station = self.stations.iter()
                .position(|s| s.id == station_id)
                .unwrap_or(0);
        }
        
        Ok(())
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        match self.api.get_info().await {
            Ok(data) => {
                let art_url = data.sched_current.as_ref()
                    .and_then(|current| current.song.as_ref())
                    .and_then(|song| song.art_url.clone());
                self.sync_data = Some(data);
                self.update_album_art(art_url).await;
                Ok(())
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e));
                Err(e)
            }
        }
    }

    async fn update_album_art(&mut self, art_url: Option<String>) {
        if art_url == self.album_art_url {
            return;
        }

        self.album_art_url = art_url.clone();
        self.album_art = None;

        if let Some(url) = art_url {
            match self.api.get_album_art(&url, 24, 24).await {
                Ok(art) => self.album_art = Some(art),
                Err(e) => self.set_status(format!("Album art unavailable: {e}")),
            }
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        match self.api.login(username, password).await {
            Ok(user_info) => {
                self.auth.user_id = Some(user_info.user_id);
                self.auth.api_key = Some(user_info.api_key.clone());
                self.auth.username = Some(user_info.username.clone());
                self.auth.save()?;
                
                self.api = RainwaveApi::with_auth(
                    user_info.user_id,
                    user_info.api_key,
                    self.auth.station_id.unwrap_or(1),
                );
                
                self.set_status(format!("Logged in as {}", user_info.username));
                self.refresh_data().await?;
                Ok(())
            }
            Err(e) => {
                self.set_status(format!("Login failed: {}", e));
                Err(e)
            }
        }
    }

    pub fn logout(&mut self) {
        self.auth.clear_secret();
        self.auth.user_id = None;
        self.auth.api_key = None;
        self.auth.username = None;
        self.auth.save().ok();
        self.api = RainwaveApi::new();
        self.set_status("Logged out".to_string());
    }

    pub async fn vote(&mut self, entry_id: i32) -> Result<()> {
        self.api.vote(entry_id).await?;
        self.set_status("Vote submitted!".to_string());
        self.refresh_data().await?;
        Ok(())
    }

    pub async fn request_song(&mut self, song_id: i32) -> Result<()> {
        self.api.request_song(song_id).await?;
        self.set_status("Song requested!".to_string());
        self.refresh_data().await?;
        Ok(())
    }

    pub async fn delete_request(&mut self, request_id: i32) -> Result<()> {
        self.api.delete_request(request_id).await?;
        self.set_status("Request removed".to_string());
        self.refresh_data().await?;
        Ok(())
    }

    pub async fn rate_song(&mut self, song_id: i32, rating: f32) -> Result<()> {
        self.api.rate_song(song_id, rating).await?;
        self.set_status(format!("Rated {:.1} stars", rating));
        self.refresh_data().await?;
        Ok(())
    }

    pub async fn fave_song(&mut self, song_id: i32, fave: bool) -> Result<()> {
        self.api.fave_song(song_id, fave).await?;
        let action = if fave { "favorited" } else { "unfavorited" };
        self.set_status(format!("Song {}", action));
        self.refresh_data().await?;
        Ok(())
    }

    pub async fn search(&mut self, query: &str) -> Result<()> {
        self.search_results = self.api.search(query).await?;
        self.selected_search = 0;
        Ok(())
    }

    pub async fn load_albums(&mut self) -> Result<()> {
        self.albums = self.api.get_all_albums().await?;
        self.selected_album = 0;
        Ok(())
    }

    pub async fn load_album_songs(&mut self, album_id: i32) -> Result<()> {
        let album = self.api.get_album(album_id).await?;
        // In a real implementation, the album API would return songs
        // For now, we'll use search as a workaround
        self.set_status(format!("Loaded album: {}", album.name));
        Ok(())
    }

    pub fn play_station(&mut self, station_id: i32) -> Result<()> {
        self.player.play(station_id)?;
        self.api.station_id = station_id;
        self.auth.station_id = Some(station_id);
        self.auth.save().ok();
        self.set_status(format!("Playing station {}", station_id));
        Ok(())
    }

    pub fn toggle_pause(&mut self) -> Result<()> {
        let station_id = self.auth.station_id.unwrap_or(1);
        self.player.toggle_pause(station_id)?;
        let status = if self.player.is_paused() {
            "Playback paused"
        } else if self.player.is_playing() {
            "Playback playing"
        } else {
            "Playback stopped"
        };
        self.set_status(status.to_string());
        Ok(())
    }

    pub fn stop_playback(&mut self) -> Result<()> {
        self.player.stop()?;
        self.set_status("Playback stopped".to_string());
        Ok(())
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, Instant::now()));
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => self.handle_search_input(key),
            InputMode::Rating => self.handle_rating_input(key),
            InputMode::LoginUsername => self.handle_login_username(key),
            InputMode::LoginPassword => self.handle_login_password(key),
            InputMode::Normal => self.handle_normal_input(key),
        }
    }

    fn handle_login_username(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.input_mode = InputMode::LoginPassword;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.current_view = AppView::NowPlaying;
                self.login_username.clear();
            }
            KeyCode::Char(c) => {
                self.login_username.push(c);
            }
            KeyCode::Backspace => {
                self.login_username.pop();
            }
            _ => {}
        }
    }

    fn handle_login_password(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let username = self.login_username.clone();
                let password = self.login_password.clone();
                let api = self.api.clone();
                tokio::spawn(async move {
                    let _ = api.login(&username, &password).await;
                });
                self.input_mode = InputMode::Normal;
                self.current_view = AppView::NowPlaying;
                self.login_username.clear();
                self.login_password.clear();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.current_view = AppView::NowPlaying;
                self.login_username.clear();
                self.login_password.clear();
            }
            KeyCode::Char(c) => {
                self.login_password.push(c);
            }
            KeyCode::Backspace => {
                self.login_password.pop();
            }
            _ => {}
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.pending_search = Some(self.search_query.clone());
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            _ => {}
        }
    }

    fn handle_rating_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Ok(rating) = self.rating_input.parse::<f32>() {
                    if rating >= 1.0 && rating <= 5.0 {
                        // Get current song ID and rate it
                        if let Some(ref data) = self.sync_data {
                            if let Some(ref current) = data.sched_current {
                                if let Some(ref song) = current.song {
                                    let song_id = song.id;
                                    let api = self.api.clone();
                                    tokio::spawn(async move {
                                        let _ = api.rate_song(song_id, rating).await;
                                    });
                                }
                            }
                        }
                    }
                }
                self.rating_input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.rating_input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) => {
                self.rating_input.push(c);
            }
            KeyCode::Backspace => {
                self.rating_input.pop();
            }
            _ => {}
        }
    }

    fn handle_normal_input(&mut self, key: KeyEvent) {
        if self.show_help && key.code == KeyCode::Esc {
            self.show_help = false;
            return;
        }

        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if matches!(self.current_view, AppView::Login) {
                    self.current_view = AppView::NowPlaying;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab => {
                self.next_view();
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = self.toggle_pause();
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = self.stop_playback();
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.auth.is_logged_in() {
                    self.set_view(AppView::Login);
                    self.input_mode = InputMode::LoginUsername;
                } else {
                    self.logout();
                }
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.set_view(AppView::Search);
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_mode = InputMode::Rating;
                self.rating_input.clear();
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Vote for selected song in election
                if let Some(ref data) = self.sync_data {
                    if let Some(ref sched) = data.sched_current {
                        let entry_id = sched.election.as_ref()
                            .and_then(|elec| elec.votes.get(self.selected_election))
                            .map(|entry| entry.song.entry_id.unwrap_or(entry.song.id))
                            .or_else(|| sched.songs.get(self.selected_election)
                                .map(|song| song.entry_id.unwrap_or(song.id)));

                        if let Some(entry_id) = entry_id {
                            let api = self.api.clone();
                            tokio::spawn(async move {
                                let _ = api.vote(entry_id).await;
                            });
                        }
                    }
                }
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Favorite current song
                if let Some(ref data) = self.sync_data {
                    if let Some(ref current) = data.sched_current {
                        if let Some(ref song) = current.song {
                            let song_id = song.id;
                            let fave = song.fav != Some(true);
                            let api = self.api.clone();
                            tokio::spawn(async move {
                                let _ = api.fave_song(song_id, fave).await;
                            });
                        }
                    }
                }
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Request current song or selected song
                if let Some(ref data) = self.sync_data {
                    if let Some(ref current) = data.sched_current {
                        if let Some(ref song) = current.song {
                            let song_id = song.id;
                            let api = self.api.clone();
                            tokio::spawn(async move {
                                let _ = api.request_song(song_id).await;
                            });
                        }
                    }
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete selected request
                if let Some(ref data) = self.sync_data {
                    if let Some(req) = data.requests.get(self.selected_request) {
                        let req_id = req.id;
                        let api = self.api.clone();
                        tokio::spawn(async move {
                            let _ = api.delete_request(req_id).await;
                        });
                    }
                }
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.set_view(AppView::Stations);
            }
            KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::Enter => {
                self.handle_enter();
            }
            _ => {}
        }
    }

    fn navigate_up(&mut self) {
        match self.current_view {
            AppView::Stations => {
                if self.selected_station > 0 {
                    self.selected_station -= 1;
                }
            }
            AppView::Requests => {
                if self.selected_request > 0 {
                    self.selected_request -= 1;
                }
            }
            AppView::Search => {
                if self.selected_search > 0 {
                    self.selected_search -= 1;
                }
            }
            AppView::Albums => {
                if self.selected_album > 0 {
                    self.selected_album -= 1;
                }
            }
            _ => {}
        }
    }

    fn navigate_down(&mut self) {
        match self.current_view {
            AppView::Stations => {
                if self.selected_station + 1 < self.stations.len() {
                    self.selected_station += 1;
                }
            }
            AppView::Requests => {
                if let Some(ref data) = self.sync_data {
                    if self.selected_request + 1 < data.requests.len() {
                        self.selected_request += 1;
                    }
                }
            }
            AppView::Search => {
                if self.selected_search + 1 < self.search_results.len() {
                    self.selected_search += 1;
                }
            }
            AppView::Albums => {
                if self.selected_album + 1 < self.albums.len() {
                    self.selected_album += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_enter(&mut self) {
        match self.current_view {
            AppView::Stations => {
                if let Some(station) = self.stations.get(self.selected_station) {
                    let _ = self.play_station(station.id);
                }
            }
            AppView::Search => {
                if let Some(song) = self.search_results.get(self.selected_search) {
                    let song_id = song.id;
                    let api = self.api.clone();
                    tokio::spawn(async move {
                        let _ = api.request_song(song_id).await;
                    });
                }
            }
            AppView::Albums => {
                if let Some(album) = self.albums.get(self.selected_album) {
                    let album = album.clone();
                    let api = self.api.clone();
                    tokio::spawn(async move {
                        let _ = api.get_album(album.id).await;
                    });
                }
            }
            _ => {}
        }
    }

    fn next_view(&mut self) {
        let next = match self.current_view {
            AppView::NowPlaying => AppView::Stations,
            AppView::Stations => AppView::Requests,
            AppView::Requests => AppView::Search,
            AppView::Search => AppView::Albums,
            AppView::Albums => AppView::NowPlaying,
            AppView::AlbumView(_) => AppView::NowPlaying,
            _ => AppView::NowPlaying,
        };
        self.set_view(next);
    }

    fn set_view(&mut self, view: AppView) {
        self.transition_label = view.label().to_string();
        self.transition_frames = 8;
        self.current_view = view;
    }
}

pub async fn run_app() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new()?;
    app.init().await?;

    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_secs(10);

    let result = run_event_loop(&mut terminal, &mut app, &mut last_refresh, refresh_interval).await;

    ratatui::restore();
    result
}

async fn run_event_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
    last_refresh: &mut Instant,
    refresh_interval: Duration,
) -> Result<()> {
    loop {
        if app.should_quit {
            break;
        }

        if last_refresh.elapsed() >= refresh_interval {
            let _ = app.refresh_data().await;
            *last_refresh = Instant::now();
        }

        app.animation_tick = app.animation_tick.wrapping_add(1);
        app.transition_frames = app.transition_frames.saturating_sub(1);

        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if let Some(query) = app.pending_search.take() {
            if !query.trim().is_empty() {
                match app.api.search(&query).await {
                    Ok(results) => {
                        app.search_results = results;
                        app.selected_search = 0;
                        app.set_status(format!("Search complete: {query}"));
                    }
                    Err(e) => app.set_status(format!("Search failed: {e}")),
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    
    let main_area = chunks[0];
    
    match &app.current_view {
        AppView::NowPlaying => {
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(main_area);
            
            render_now_playing(f, inner_chunks[0], &app.sync_data, app.album_art.as_ref(), app.animation_tick,
                &app.stations.get(app.selected_station)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()));
            
            let election = app.sync_data.as_ref()
                .and_then(|d| d.sched_current.as_ref())
                .and_then(|s| {
                    s.election.clone().or_else(|| {
                        if s.songs.is_empty() {
                            None
                        } else {
                            Some(Election {
                                id: s.id,
                                start_actual: s.start_actual,
                                end_actual: s.end_actual,
                                votes: s.songs.iter().cloned().map(|song| VoteEntry {
                                    votes: song.votes,
                                    voted: false,
                                    song,
                                }).collect(),
                            })
                        }
                    })
                });
            render_election(f, inner_chunks[1], &election);
        }
        AppView::Stations => {
            render_stations(f, main_area, &app.stations, app.selected_station);
        }
        AppView::Requests => {
            render_request_queue(f, main_area, 
                &app.sync_data.as_ref().map(|d| d.requests.clone()).unwrap_or_default());
        }
        AppView::Search => {
            render_search(f, main_area, &app.search_query, &app.search_results, app.selected_search);
        }
        AppView::Albums => {
            render_albums(f, main_area, &app.albums, app.selected_album);
        }
        AppView::AlbumView(album) => {
            let block = styled_block(&format!(" Album: {} ", album.name), Color::Green);
            f.render_widget(block, main_area);
        }
        AppView::Help => {
            render_help(f, main_area);
        }
        AppView::Login => {
            render_login(f, main_area, &app.login_username, app.login_password.len());
        }
    }
    
    let station_name = app.stations.get(app.selected_station)
        .map(|s| s.name.as_str())
        .unwrap_or("None");
    
    let user_info = if app.auth.is_logged_in() {
        Some(UserInfo {
            user_id: app.auth.user_id.unwrap_or(0),
            username: app.auth.username.clone().unwrap_or_default(),
            api_key: String::new(),
            listener_points: 0,
            avatar: None,
        })
    } else {
        None
    };
    
    render_status_bar(
        f,
        chunks[1],
        &user_info,
        app.player.is_playing(),
        app.player.is_paused(),
        app.animation_tick,
        station_name,
    );
    
    let view_name = match app.current_view {
        AppView::NowPlaying => "NowPlaying",
        AppView::Stations => "Stations",
        AppView::Requests => "Requests",
        AppView::Search => "Search",
        AppView::Albums => "Albums",
        AppView::AlbumView(_) => "AlbumView",
        AppView::Help => "Help",
        AppView::Login => "Login",
    };
    render_keybindings(f, chunks[2], view_name);
    
    if app.show_help {
        render_help(f, centered_rect(60, 70, f.area()));
    }

    if app.transition_frames > 0 {
        render_transition(f, centered_rect(42, 16, f.area()), &app.transition_label, app.animation_tick);
    }
    
    if matches!(app.input_mode, InputMode::Rating) {
        render_rating_popup(f, app);
    }
}

fn render_rating_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(30, 20, f.area());
    let block = styled_block(" Rate Song (1.0-5.0) ", Color::Yellow);
    let inner = block.inner(area);
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);
    
    let input = Paragraph::new(app.rating_input.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(input, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
