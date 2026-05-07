# Rainwave TUI Design

## Goal

Rainwave TUI is a Rust terminal interface for Rainwave that provides playback control, station browsing, now-playing metadata, voting, requests, ratings, favorites, login, and search from a keyboard-first terminal workflow.

## Technology

- Rust async runtime: `tokio`
- TUI renderer: `ratatui` with `crossterm`
- HTTP client: `reqwest`
- JSON parsing: `serde` and `serde_json`
- External playback: `mpv`, `mplayer`, or `ffplay`
- Credential storage: OS keyring via `keyring`
- Demo asset generation: `ffmpeg`-generated GIF under `docs/rainwave-tui-demo.gif`

## Core Modules

- `src/main.rs`: process entrypoint and async runtime.
- `src/api.rs`: Rainwave API client, endpoint calls, response normalization.
- `src/models.rs`: tolerant API data models for Rainwave's live JSON shapes.
- `src/player.rs`: external audio-player process management.
- `src/auth.rs`: local auth/config persistence under `~/.config/rainwave-tui/config.json`.
- `src/ui/app.rs`: application state, key handling, event loop, view selection.
- `src/ui/views.rs`: rendering for now playing, voting, requests, stations, search, albums, login, and help.
- `src/ui/widgets.rs`: shared visual helpers.

## Credential Storage

Rainwave passwords are never stored. After login, only the returned API key is kept for future authenticated calls.

The API key is stored in the operating system keyring using the `keyring` crate under service `rainwave-tui`. The JSON config at `~/.config/rainwave-tui/config.json` stores only non-secret metadata:

- `user_id`
- `username`
- `station_id`

If an older config contains a plaintext `api_key`, the loader migrates it into the keyring and rewrites the JSON without the secret. Logout deletes the keyring entry.

If the system keyring is unavailable, login persistence fails instead of falling back to plaintext storage.

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

- Now Playing: album-art panel, current song metadata, animated music glyphs, elapsed/remaining progress, listener/status data, election voting list.
- Stations: station list and station switching.
- Requests: request-line display and delete action.
- Search: search input and request-from-results flow.
- Albums: album listing placeholder for future deep browsing.
- Login: username/password input.
- Help: command reference overlay.

## Animation And Status Feedback

- Screen changes trigger a short centered transition overlay with spinner/arrow animation.
- The now-playing pane cycles music-glyph frames while data is displayed.
- The status bar shows an audio state icon and animated meter:
  - Playing: green animated level meter
  - Paused: pause icon and flat meter
  - Stopped: stop icon and inactive meter
- Help/input overlays close with `Esc`.

## Album Art

Rainwave exposes album art paths under `songs[].albums[].art`. The API normalization layer turns relative art paths into full `https://rainwave.cc/...` URLs and stores them on `Song::art_url`.

The app fetches the current album image through `reqwest`, decodes it with the `image` crate, downsamples it to a small square, and renders it with upper-half block characters using RGB foreground/background colors. This avoids requiring Kitty/Sixel/iTerm graphics support while still showing real album art in normal color-capable terminals.

If image fetch or decoding fails, the panel falls back to a styled placeholder with the album name and resolved art URL.

## Search Flow

`Ctrl+K` switches to the Search view and enters search input mode. Typed characters update the query shown in the search box. Pressing `Enter` queues the query for the main event loop, which awaits `api.search()` and updates `search_results` in app state. This avoids the earlier detached task behavior where results were fetched and discarded.

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

## README Demo GIF

The README includes `docs/rainwave-tui-demo.gif`. Because this environment only had `ffmpeg` available and no interactive terminal GIF recorder (`vhs`, `asciinema`, `agg`, or `terminalizer`), the GIF is generated from scripted terminal-style frames that demonstrate the intended UI states:

- Now Playing layout
- Album art panel
- Audio activity meter
- Progress bar
- Screen-switch transition overlay

Future improvement: replace it with a real terminal recording once a recorder is available.

## Current Changes Since Initial Commit

- Added keyring-backed credential storage and plaintext API-key migration.
- Added Ctrl-only command scheme.
- Added play/pause/resume and stop controls.
- Added animated transitions and audio activity indicator.
- Added album-art metadata extraction and Now Playing art panel.
- Added actual album-art HTTP fetching, image decoding, downsampling, and terminal color-block rendering.
- Fixed search text input and result population.
- Added progress text and progress bar improvements.
- Added README demo GIF and synchronized README/design documentation.
- Added regression tests for live Rainwave JSON shapes and null handling.

## Known Follow-Ups

- Replace scripted README GIF with a real terminal recording when recording tooling is available.
- Wire login completion back into app state instead of spawning detached login requests.
- Complete album browsing and request/search result state updates.
- Remove unused imports and dead-code warnings after the feature surface settles.
