# Rainwave TUI Design

## Goal

Rainwave TUI is a Rust terminal interface for Rainwave that provides playback control, station browsing, now-playing metadata, voting, requests, ratings, favorites, login, and search from a keyboard-first terminal workflow.

## Technology

- Rust async runtime: `tokio`
- TUI renderer: `ratatui` with `crossterm`
- HTTP client: `reqwest`
- JSON parsing: `serde` and `serde_json`
- External playback: `mpv`, `mplayer`, or `ffplay`

## Core Modules

- `src/main.rs`: process entrypoint and async runtime.
- `src/api.rs`: Rainwave API client, endpoint calls, response normalization.
- `src/models.rs`: tolerant API data models for Rainwave's live JSON shapes.
- `src/player.rs`: external audio-player process management.
- `src/auth.rs`: local auth/config persistence under `~/.config/rainwave-tui/config.json`.
- `src/ui/app.rs`: application state, key handling, event loop, view selection.
- `src/ui/views.rs`: rendering for now playing, voting, requests, stations, search, albums, login, and help.
- `src/ui/widgets.rs`: shared visual helpers.

## API Shape Handling

Rainwave responses are not fully uniform. The models are intentionally tolerant of:

- Alternate field names, such as `id` for `user_id`, `display_name` for username, `stream` for station URL, and `key` for station genre.
- Explicit `null` values in fields such as `request_line[].song_id`, `request_line[].song`, and schedule timestamps.
- Song metadata nested under `sched_current.songs[]` instead of `sched_current.song`.
- Album art nested under `songs[].albums[].art`.

`api.rs` normalizes schedule songs after parsing so the UI can read a current song consistently.

## Playback

Playback launches an external player process against Rainwave stream URLs:

- Preferred: `mpv`
- Fallbacks: `mplayer`, `ffplay`

The player module tracks whether the subprocess is active or paused. Pause/resume uses POSIX `SIGSTOP`/`SIGCONT`; stop kills the process.

## UI Views

- Now Playing: album-art panel, current song metadata, progress, listener/status data, election voting list.
- Stations: station list and station switching.
- Requests: request-line display and delete action.
- Search: search input and request-from-results flow.
- Albums: album listing placeholder for future deep browsing.
- Login: username/password input.
- Help: command reference overlay.

## Controls

Commands are Ctrl-based to avoid accidental actions while navigating.

- `Tab`: switch views
- `Up` / `Down`: move selection
- `Enter`: select current item
- `Esc`: close help or cancel input
- `Ctrl+P`: play/pause/resume
- `Ctrl+O`: stop playback
- `Ctrl+H`: toggle help
- `Ctrl+Q`: quit
- `Ctrl+L`: login/logout
- `Ctrl+K`: search
- `Ctrl+V`: vote
- `Ctrl+T`: rate current song
- `Ctrl+F`: favorite current song
- `Ctrl+R`: request current/selected song
- `Ctrl+D`: delete selected request
- `Ctrl+B`: browse stations

## Verification

Current verification commands:

```bash
cargo test
cargo build
```

Regression tests cover the live Rainwave `/info` and `/stations` JSON shapes that previously caused launch failures.

## Known Follow-Ups

- Replace album-art URL placeholder with actual terminal image rendering where terminal support is available.
- Wire login completion back into app state instead of spawning detached login requests.
- Complete album browsing and request/search result state updates.
- Remove unused imports and dead-code warnings after the feature surface settles.
