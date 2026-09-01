# rainwave-tui

A gorgeous terminal client for [Rainwave](https://rainwave.cc) — community-driven video game music radio.

```
rainwave-tui v0.1.0 — Press ? for help
```

## Features

- **6 radio stations**: Game, OC ReMix, Covers, Chiptunes, All, Chill
- **Album art** rendered as colored Unicode half-blocks in your terminal
- **Animated rain** effect background (toggle with `r`)
- **9 color themes**: Ocean, Dracula, Nord, Monokai, Solarized Dark, Catppuccin Mocha, Gruvbox, Tokyo Night, Rosé Pine
- **Dvorak & Colemak** keyboard layout support — same physical keys, remapped shortcuts
- **Persistent config** — saves theme, volume, station, layout to `~/.config/rainwave-tui/config.json`
- **Desktop notifications** via `notify-send` on song change
- **Clipboard support** — copy song info with `y` (needs `xclip` or `xsel`)
- **Song history** — tracks what you've heard this session
- **Song & artist detail panel** — press `i` for full info including album, rating, artist IDs
- **Volume control** with `+`/`-` (mpv restarts with new volume)
- **Rating as stars** — colored ★☆ visualization on all panels
- **Dual-color progress bar** with elapsed/remaining time
- **Interactive options** — change settings in-app with live preview
- **Login via OS keyring** — credentials stored securely with `keyring` crate
- **mpv playback** with play/pause toggle

## Requirements

- [Rust](https://rustup.rs/) (edition 2024)
- [mpv](https://mpv.io/) (for audio playback)
- `xclip` or `xsel` (optional, for clipboard)
- `libnotify` / `notify-send` (optional, for desktop notifications)
- Linux with Secret Service / GNOME Keyring (for credential storage)

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
git clone <repo-url>
cd rainwave-tui
cargo build --release
```

The binary will be at `target/release/rainwave-tui`.

## Usage

```bash
rainwave-tui          # launch the TUI
rainwave-tui --version # print version
```

On first launch, press `l` to log in with your Rainwave user ID and API key. Credentials are stored in your OS keyring.

## Keybindings

### Navigation
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle sections |
| `↑`/`↓` / layout key | Move selection |
| `Enter` | Activate selected |
| `?` | Toggle help overlay |
| `Esc` | Close help / panels |

### Playback
| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `1`-`6` | Switch station |
| `R` | Refresh now playing |
| `+`/`=` / `-` | Volume up / down |

### Actions (QWERTY — see `?` help for your layout)
| Key | Action |
|-----|--------|
| `v` | Vote on selected entry |
| `x` | Request selected song |
| `d` | Delete selected request |
| `c` | Clear request queue |
| `0`-`5` | Rate song (0 = clear) |
| `f` | Toggle favorite |

### Search & Browse
| Key | Action |
|-----|--------|
| `/` | Search the catalog |
| `a` | Browse album by ID |

### Info & Tools
| Key | Action |
|-----|--------|
| `i` | Song & artist info panel |
| `y` | Copy song info to clipboard |
| `h` | View song history |

### Account
| Key | Action |
|-----|--------|
| `l` | Log in (OS keyring) |
| `o` | Log out & clear keyring |

### Options
| Key | Action |
|-----|--------|
| `O` / `F1` | Open options menu |
| `r` | Toggle rain effect |
| `q` | Quit |

## Keyboard Layouts

rainwave-tui ships with three keyboard layout presets that remap shortcuts to the same **physical key positions**:

- **QWERTY** (default)
- **Dvorak** — press where the QWERTY key would be
- **Colemak**

Switch via the Options menu (`O` → Layout tab) or the config file. The help overlay always shows the correct keys for your active layout.

## Configuration

Config is saved to `~/.config/rainwave-tui/config.json`:

```json
{
  "theme_idx": 0,
  "refresh_interval": 15,
  "mpv_binary": "mpv",
  "rain_show": true,
  "volume": 100,
  "last_station": 1,
  "layout": "qwerty",
  "notifications": true
}
```

## License

MIT
