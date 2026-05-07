use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub user_id: Option<i32>,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub station_id: Option<i32>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            user_id: None,
            api_key: None,
            username: None,
            station_id: Some(1),
        }
    }
}

impl AuthConfig {
    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
        let path = home.join(".config").join("rainwave-tui").join("config.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(path)
    }

    pub fn load() -> Self {
        Self::config_path()
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn is_logged_in(&self) -> bool {
        self.user_id.is_some() && self.api_key.is_some()
    }
}
