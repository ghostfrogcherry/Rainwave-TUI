# Rainwave TUI

A flashy terminal user interface for Rainwave radio stations.

## Features

- 🎵 **Now Playing** - Shows current song, artist, album, and progress
- 📻 **Station Selection** - Browse and switch between Rainwave stations
- 🗳️ **Voting** - Vote for songs in elections
- 📋 **Request Queue** - Manage your song requests
- 🔍 **Search** - Search for songs, albums, and artists
- 💿 **Album Browsing** - Browse albums and their songs
- 🔐 **Login Support** - Login to access all features
- ▶️ **Playback** - Audio playback via mpv/mplayer/ffplay
- ⭐ **Rating** - Rate songs (1.0-5.0)
- 💖 **Favorites** - Favorite/unfavorite songs

## Installation

```bash
git clone <repo-url>
cd Rainwave-TUI
cargo build --release
```

## Usage

Run the TUI:
```bash
cargo run
```

## Keybindings

### Navigation
- `Tab` - Switch between views
- `↑/↓` - Navigate lists
- `Enter` - Select item / Vote / Request song

### Actions
- `v` - Vote for selected song in election
- `r` - Rate current song (opens rating input)
- `f` - Favorite/unfavorite current song
- `R` - Add current song to request queue
- `d` - Delete selected request

### Playback
- `p` or `Space` - Play/Pause
- `s` - Switch station (use Tab to go to Stations view)

### Other
- `l` - Login/Logout
- `/` - Search
- `?` - Toggle help
- `q` - Quit

## Requirements

- Rust (for building)
- One of: mpv, mplayer, or ffplay (for audio playback)
- Terminal with color support

## Configuration

Login credentials are stored in `~/.config/rainwave-tui/config.json`

## Colors & Styling

The TUI uses colors to indicate different elements:
- 🔵 Cyan - Now Playing, Search
- 🟡 Yellow - Elections, User info
- 🟢 Green - Requests, Albums
- 🔴 Magenta - Progress bars
- 🔵 Blue - Stations
- 🟣 Purple - Login screen
