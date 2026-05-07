use anyhow::{Result, anyhow};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct Player {
    process: Arc<Mutex<Option<Child>>>,
    station_id: i32,
    playing: bool,
    paused: bool,
}

impl Player {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            station_id: 1,
            playing: false,
            paused: false,
        }
    }

    pub fn play(&mut self, station_id: i32) -> Result<()> {
        self.stop()?;
        self.station_id = station_id;
        
        let url = format!("https://rainwave.cc/tune_in/{}.mp3", station_id);
        
        // Try mpv first, then mplayer, then ffplay
        let players = vec![
            ("mpv", vec!["--no-video", "--quiet", &url]),
            ("mplayer", vec!["-quiet", "-nolirc", &url]),
            ("ffplay", vec!["-nodisp", "-loglevel", "quiet", &url]),
        ];
        
        for (player, args) in players {
            if Command::new(player).arg("--version").output().is_ok() {
                let child = Command::new(player)
                    .args(&args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
                
                *self.process.lock().unwrap() = Some(child);
                self.playing = true;
                self.paused = false;
                return Ok(());
            }
        }
        
        Err(anyhow!("No audio player found. Install mpv, mplayer, or ffplay"))
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.playing = false;
        self.paused = false;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        if !self.playing || self.paused {
            return Ok(());
        }

        if let Some(child) = self.process.lock().unwrap().as_ref() {
            Command::new("kill")
                .args(["-STOP", &child.id().to_string()])
                .status()?;
            self.paused = true;
        }
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        if !self.playing || !self.paused {
            return Ok(());
        }

        if let Some(child) = self.process.lock().unwrap().as_ref() {
            Command::new("kill")
                .args(["-CONT", &child.id().to_string()])
                .status()?;
            self.paused = false;
        }
        Ok(())
    }

    pub fn toggle_pause(&mut self, station_id: i32) -> Result<()> {
        if self.playing && self.paused {
            self.resume()
        } else if self.playing {
            self.pause()
        } else {
            self.play(station_id)
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn station_id(&self) -> i32 {
        self.station_id
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
