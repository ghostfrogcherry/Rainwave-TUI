use crate::models::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    text::{Line, Span, Text},
    Frame,
};
use crate::ui::widgets::*;

pub fn render_now_playing(f: &mut Frame, area: Rect, sync_data: &Option<SyncData>, station_name: &str) {
    let block = styled_block(&format!(" 🎵 Now Playing - {} ", station_name), Color::Cyan);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(data) = sync_data {
        if let Some(current) = &data.sched_current {
            if let Some(song) = &current.song {
                let columns = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(30), Constraint::Min(0)])
                    .split(inner);

                render_album_art(f, columns[0], song);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(1),
                        Constraint::Length(2),
                        Constraint::Min(0),
                    ])
                    .split(columns[1]);

                // Visual indicator
                let indicator = Paragraph::new("♪♫♬ ♭♮♯ ")
                    .style(Style::default().fg(Color::Magenta))
                    .alignment(Alignment::Center);
                f.render_widget(indicator, chunks[0]);

                // Song info
                let title = Paragraph::new(Text::from(vec![
                    Line::from(vec![
                        Span::styled("♪ ", Style::default().fg(Color::Yellow)),
                        Span::styled(&song.title, Style::default().fg(Color::White).bold()),
                    ]),
                    Line::from(vec![
                        Span::styled("  by ", Style::default().fg(Color::Gray)),
                        Span::styled(&song.artist, Style::default().fg(Color::Cyan)),
                    ]),
                ]))
                .block(Block::default().borders(Borders::NONE));
                f.render_widget(title, chunks[1]);

                // Album and rating
                let info = Paragraph::new(Text::from(vec![
                    Line::from(vec![
                        Span::styled("  └─ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            song.album.as_deref().unwrap_or("Unknown Album"),
                            Style::default().fg(Color::Green)
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format_rating(song.rating_user),
                            Style::default().fg(Color::Yellow)
                        ),
                    ]),
                ]))
                .block(Block::default().borders(Borders::NONE));
                f.render_widget(info, chunks[2]);

                // Listener count
                if let Some(listeners) = data.listener_count {
                    let listeners_text = Paragraph::new(format!("👂 {} listeners", listeners))
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Right);
                    f.render_widget(listeners_text, chunks[3]);
                }

                // Progress bar
                if current.start_actual > 0 && current.end_actual > 0 {
                    let now = chrono::Utc::now().timestamp();
                    let total = current.end_actual - current.start_actual;
                    let elapsed = now - current.start_actual;
                    let progress = if total > 0 { elapsed as f64 / total as f64 } else { 0.0 }
                        .min(1.0).max(0.0);
                    
                    let remaining = (current.end_actual - now).max(0) as i32;
                    let progress_text = Paragraph::new(format!(
                        "{} elapsed / {} remaining",
                        format_duration(elapsed.max(0) as i32),
                        format_duration(remaining),
                    ))
                    .style(Style::default().fg(Color::DarkGray));
                    f.render_widget(progress_text, chunks[4]);

                    let gauge = progress_bar(progress, Color::Magenta);
                    f.render_widget(gauge, chunks[5]);
                }
            }
        }
    } else {
        let loading = Paragraph::new("⏳ Loading...")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(loading, inner);
    }
}

fn render_album_art(f: &mut Frame, area: Rect, song: &Song) {
    let block = styled_block(" Album Art ", Color::Magenta);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let album = song.album.as_deref().unwrap_or("Unknown Album");
    let art = song.art_url.as_deref().unwrap_or("No album art URL");
    let text = Text::from(vec![
        Line::from("╔════════════════════╗"),
        Line::from("║                    ║"),
        Line::from("║      ALBUM ART     ║"),
        Line::from("║                    ║"),
        Line::from("╚════════════════════╝"),
        Line::from(""),
        Line::from(vec![Span::styled(album.to_string(), Style::default().fg(Color::Green).bold())]),
        Line::from(""),
        Line::from(vec![Span::styled(art.to_string(), Style::default().fg(Color::DarkGray))]),
    ]);

    let art_panel = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(art_panel, inner);
}

pub fn render_election(f: &mut Frame, area: Rect, election: &Option<Election>) {
    let block = styled_block(" 🗳️  Vote Now! ", Color::Yellow);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(elec) = election {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .split(inner);

        let time_left = if elec.end_actual > 0 {
            format_time_remaining(elec.end_actual)
        } else {
            "N/A".to_string()
        };
        
        let header = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("Time left: ", Style::default().fg(Color::Gray)),
                Span::styled(time_left, Style::default().fg(Color::Green).bold()),
            ]),
        ]));
        f.render_widget(header, chunks[0]);

        let rows: Vec<Row> = elec.votes.iter().enumerate().map(|(i, entry)| {
            let style = if entry.voted {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default()
            };
            
            Row::new(vec![
                Cell::from(format!("{}", i + 1)),
                Cell::from(entry.song.title.clone()),
                Cell::from(entry.song.artist.clone()),
                Cell::from(format!("{} votes", entry.votes)),
                Cell::from(if entry.voted { "✓".to_string() } else { " ".to_string() }),
            ]).style(style)
        }).collect();

        let table = Table::new(rows, &[
            Constraint::Length(3),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Length(12),
            Constraint::Length(3),
        ])
        .header(table_header())
        .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, chunks[1]);
    } else {
        let no_elec = Paragraph::new("No election in progress")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(no_elec, inner);
    }
}

pub fn render_request_queue(f: &mut Frame, area: Rect, requests: &[RequestEntry]) {
    let block = styled_block(" 📋 Request Queue ", Color::Green);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if requests.is_empty() {
        let empty = Paragraph::new("Request queue is empty")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty, inner);
    } else {
        let rows: Vec<Row> = requests.iter().enumerate().map(|(i, req)| {
            Row::new(vec![
                Cell::from(format!("#{}", i + 1)),
                Cell::from(req.song.title.clone()),
                Cell::from(req.song.artist.clone()),
                Cell::from(format_duration(req.song.length)),
            ])
        }).collect();

        let table = Table::new(rows, &[
            Constraint::Length(4),
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Length(10),
        ])
        .header(table_header())
        .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, inner);
    }
}

pub fn render_stations(f: &mut Frame, area: Rect, stations: &[Station], selected: usize) {
    let block = styled_block(" 📻 Stations ", Color::Blue);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = stations.iter().enumerate().map(|(i, s)| {
        let style = if i == selected {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default()
        };
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(&s.name, Style::default().fg(Color::Cyan).bold()),
                Span::raw(" - "),
                Span::styled(&s.genre, Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled(&s.description, Style::default().fg(Color::DarkGray)),
            ]),
        ]).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_widget(list, inner);
}

pub fn render_help(f: &mut Frame, area: Rect) {
    let block = styled_block(" ❓ Help ", Color::Magenta);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let help_text = vec![
        Line::from(vec![Span::styled("Navigation", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  Tab         - Switch tab"),
        Line::from("  ↑/↓         - Navigate list"),
        Line::from("  Enter       - Select/Vote/Request"),
        Line::from("  Esc         - Close help/input"),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  Ctrl+V      - Vote for selected song"),
        Line::from("  Ctrl+T      - Rate song (1-5)"),
        Line::from("  Ctrl+F      - Favorite song"),
        Line::from("  Ctrl+R      - Add to request queue"),
        Line::from("  Ctrl+D      - Delete request"),
        Line::from(""),
        Line::from(vec![Span::styled("Playback", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  Ctrl+P      - Play/Pause"),
        Line::from("  Ctrl+O      - Stop"),
        Line::from("  Ctrl+B      - Browse stations"),
        Line::from(""),
        Line::from(vec![Span::styled("Other", Style::default().fg(Color::Yellow).bold())]),
        Line::from("  Ctrl+L      - Login/Logout"),
        Line::from("  Ctrl+K      - Search"),
        Line::from("  Ctrl+Q      - Quit"),
        Line::from("  Ctrl+H      - Toggle help"),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    f.render_widget(help, inner);
}

pub fn render_login(f: &mut Frame, area: Rect, username: &str, password_len: usize) {
    let block = styled_block(" 🔐 Login ", Color::Yellow);
    let inner = block.inner(area);
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);
    
    let username_label = Paragraph::new("Username:")
        .style(Style::default().fg(Color::Gray));
    f.render_widget(username_label, chunks[0]);
    
    let username_input = Paragraph::new(username)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(username_input, chunks[1]);
    
    let password_label = Paragraph::new("Password:")
        .style(Style::default().fg(Color::Gray));
    f.render_widget(password_label, chunks[2]);
    
    let password_display = "*".repeat(password_len);
    let password_input = Paragraph::new(password_display)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(password_input, chunks[3]);
}

pub fn render_search(f: &mut Frame, area: Rect, query: &str, results: &[Song], selected: usize) {
    let block = styled_block(" 🔍 Search ", Color::Cyan);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    let search_box = Paragraph::new(format!("> {}", query))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Search Query"));
    f.render_widget(search_box, chunks[0]);

    if results.is_empty() {
        let no_results = Paragraph::new("No results found. Type to search...")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        let rows: Vec<Row> = results.iter().enumerate().map(|(i, song)| {
            let style = if i == selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(create_song_row(song)).style(style)
        }).collect();

        let table = Table::new(rows, &[
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .header(table_header())
        .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, chunks[1]);
    }
}

pub fn render_albums(f: &mut Frame, area: Rect, albums: &[Album], selected: usize) {
    let block = styled_block(" 💿 Albums ", Color::Green);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if albums.is_empty() {
        let loading = Paragraph::new("Loading albums...")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(loading, inner);
        return;
    }

    let items: Vec<ListItem> = albums.iter().enumerate().map(|(i, album)| {
        let style = if i == selected {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default()
        };
        
        let rating_str = album.rating_user
            .map(|r| format!("{:.1}★", r))
            .unwrap_or_else(|| "-".to_string());
        
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(&album.name, Style::default().fg(Color::Cyan).bold()),
                Span::raw(" "),
                Span::styled(rating_str, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled(&album.artist, Style::default().fg(Color::Gray)),
                Span::raw(" - "),
                Span::styled(
                    format!("{} songs", album.song_count.unwrap_or(0)),
                    Style::default().fg(Color::DarkGray)
                ),
            ]),
        ]).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(list, inner);
}

pub fn render_status_bar(f: &mut Frame, area: Rect, user: &Option<UserInfo>, playing: bool, paused: bool, station: &str) {
    let play_icon = if paused { "⏸" } else if playing { "▶" } else { "■" };
    let user_str = user.as_ref()
        .map(|u| format!("{} ({} pts)", u.username, u.listener_points))
        .unwrap_or_else(|| "Not logged in (Ctrl+L to login)".to_string());
    
    let status = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled(play_icon, Style::default().fg(Color::Green).bold()),
            Span::raw(" "),
            Span::styled(station, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled(user_str, Style::default().fg(Color::Yellow)),
        ]),
    ]))
    .block(Block::default().borders(Borders::TOP))
    .style(Style::default().bg(Color::DarkGray));
    f.render_widget(status, area);
}

pub fn render_keybindings(f: &mut Frame, area: Rect, view_name: &str) {
    let bindings = match view_name {
        "NowPlaying" => "[Tab] Switch [Ctrl+P] Play/Pause [Ctrl+O] Stop [Ctrl+V] Vote [Ctrl+H] Help [Ctrl+Q] Quit",
        "Stations" => "[Tab] Switch [Ctrl+P] Play/Pause [Ctrl+O] Stop [Enter] Select [Ctrl+H] Help [Ctrl+Q] Quit",
        "Requests" => "[Tab] Switch [Ctrl+D] Delete [Ctrl+R] Request [Ctrl+H] Help [Ctrl+Q] Quit",
        "Search" => "[Tab] Switch [Ctrl+K] Search [Enter] Request [Ctrl+H] Help [Ctrl+Q] Quit",
        "Albums" => "[Tab] Switch [Enter] Select [Ctrl+H] Help [Ctrl+Q] Quit",
        "Login" => "[Enter] Next [Esc] Cancel",
        _ => "[Tab] Switch [Ctrl+H] Help [Ctrl+Q] Quit",
    };
    
    let bar = Paragraph::new(bindings)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(bar, area);
}
