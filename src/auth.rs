use anyhow::{Result, anyhow};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "rainwave-tui";

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub user_id: Option<i32>,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub station_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct StoredConfig {
    user_id: Option<i32>,
    api_key: Option<String>,
    username: Option<String>,
    station_id: Option<i32>,
}

#[derive(Debug, Serialize)]
struct StoredConfigOut<'a> {
    user_id: Option<i32>,
    username: &'a Option<String>,
    station_id: Option<i32>,
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
        let Some((path, stored)) = Self::config_path()
            .ok()
            .and_then(|p| fs::read_to_string(&p).ok().map(|s| (p, s)))
            .and_then(|(p, s)| serde_json::from_str::<StoredConfig>(&s).ok().map(|c| (p, c)))
        else {
            return Self::default();
        };

        let mut config = Self {
            user_id: stored.user_id,
            api_key: None,
            username: stored.username,
            station_id: stored.station_id.or(Some(1)),
        };

        if let Some(api_key) = stored.api_key {
            config.api_key = Some(api_key);
            let _ = config.save();
        } else if let Some(user_id) = config.user_id {
            config.api_key = Self::load_api_key(user_id).ok();
        }

        let _ = path;
        config
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(user_id) = self.user_id {
            if let Some(api_key) = &self.api_key {
                Self::store_api_key(user_id, api_key)?;
            } else {
                Self::delete_api_key(user_id);
            }
        }

        let stored = StoredConfigOut {
            user_id: self.user_id,
            username: &self.username,
            station_id: self.station_id,
        };
        let content = serde_json::to_string_pretty(&stored)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn clear_secret(&self) {
        if let Some(user_id) = self.user_id {
            Self::delete_api_key(user_id);
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.user_id.is_some() && self.api_key.is_some()
    }

    fn keyring_entry(user_id: i32) -> Result<Entry> {
        Entry::new(KEYRING_SERVICE, &format!("user-{user_id}"))
            .map_err(|e| anyhow!("Could not access system keyring: {e}"))
    }

    fn load_api_key(user_id: i32) -> Result<String> {
        Self::keyring_entry(user_id)?
            .get_password()
            .map_err(|e| anyhow!("Could not read Rainwave API key from system keyring: {e}"))
    }

    fn store_api_key(user_id: i32, api_key: &str) -> Result<()> {
        Self::keyring_entry(user_id)?
            .set_password(api_key)
            .map_err(|e| anyhow!("Could not store Rainwave API key in system keyring: {e}"))
    }

    fn delete_api_key(user_id: i32) {
        if let Ok(entry) = Self::keyring_entry(user_id) {
            let _ = entry.delete_credential();
        }
    }
}
