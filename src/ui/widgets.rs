use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, LineGauge, Row},
};

pub fn styled_block(title: &str, color: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title.to_string())
}

pub fn progress_bar(percent: f64, color: Color) -> LineGauge<'static> {
    LineGauge::default()
        .ratio(percent)
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray))
}

pub fn format_duration(seconds: i32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

pub fn format_time_remaining(end_time: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let remaining = end_time - now;
    if remaining > 0 {
        format_duration(remaining as i32)
    } else {
        "0:00".to_string()
    }
}

pub fn create_song_row(song: &crate::models::Song) -> Vec<Cell<'static>> {
    vec![
        Cell::from(song.title.clone()),
        Cell::from(song.artist.clone()),
        Cell::from(song.album.clone().unwrap_or_default()),
        Cell::from(format_rating(song.rating_user)),
        Cell::from(format_duration(song.length)),
    ]
}

pub fn format_rating(rating: Option<f32>) -> String {
    match rating {
        Some(r) => format!("{:.1}★", r),
        None => "-".to_string(),
    }
}

pub fn header_style() -> Style {
    Style::default().fg(Color::Yellow).bold()
}

pub fn table_header() -> Row<'static> {
    Row::new(vec!["Title", "Artist", "Album", "Rating", "Length"])
        .style(header_style())
        .height(1)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
