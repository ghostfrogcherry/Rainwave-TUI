use serde::{Deserialize, Serialize};

fn de_null_i64_default<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<i64>::deserialize(deserializer)?.unwrap_or_default())
}

fn de_null_i32_default<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.unwrap_or_default())
}

fn de_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Station {
    #[serde(default, deserialize_with = "de_null_i32_default")]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, alias = "stream")]
    pub stream_url: String,
    #[serde(default, alias = "key")]
    pub genre: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Song {
    #[serde(default, deserialize_with = "de_null_i32_default")]
    pub id: i32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default, alias = "album_name")]
    pub album: Option<String>,
    #[serde(default)]
    pub length: i32,
    #[serde(default)]
    pub rating: Option<f32>,
    #[serde(default)]
    pub rating_user: Option<f32>,
    #[serde(default)]
    pub fav: Option<bool>,
    #[serde(default)]
    pub elec_ended: Option<i64>,
    #[serde(default)]
    pub elec_started: Option<i64>,
    #[serde(default)]
    pub entry_id: Option<i32>,
    #[serde(default, alias = "entry_votes")]
    pub votes: i32,
    #[serde(default)]
    pub art_url: Option<String>,
    #[serde(default)]
    pub artists: Vec<SongArtist>,
    #[serde(default)]
    pub albums: Vec<SongAlbum>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongArtist {
    #[serde(default, deserialize_with = "de_null_i32_default")]
    pub id: i32,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongAlbum {
    #[serde(default, deserialize_with = "de_null_i32_default")]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub art: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Election {
    pub id: i32,
    pub start_actual: i64,
    pub end_actual: i64,
    pub votes: Vec<VoteEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteEntry {
    pub song: Song,
    pub votes: i32,
    pub voted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEntry {
    #[serde(default, alias = "song_id", deserialize_with = "de_null_i32_default")]
    pub id: i32,
    #[serde(default, deserialize_with = "de_null_default")]
    pub song: Song,
    #[serde(default)]
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    #[serde(default, alias = "id")]
    pub user_id: i32,
    #[serde(default, alias = "display_name")]
    pub username: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub listener_points: i32,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    #[serde(default)]
    pub id: i32,
    #[serde(default, rename = "type")]
    pub sched_type: String,
    #[serde(default, deserialize_with = "de_null_i64_default")]
    pub start_actual: i64,
    #[serde(default, alias = "end", deserialize_with = "de_null_i64_default")]
    pub end_actual: i64,
    #[serde(default)]
    pub song: Option<Song>,
    #[serde(default)]
    pub songs: Vec<Song>,
    #[serde(default)]
    pub election: Option<Election>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: i32,
    pub name: String,
    pub artist: String,
    pub year: Option<i32>,
    pub rating: Option<f32>,
    pub rating_user: Option<f32>,
    pub fav: Option<bool>,
    pub song_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: i32,
    pub name: String,
    pub album_count: Option<i32>,
    pub song_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub result: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    #[serde(default)]
    pub station_id: Option<i32>,
    #[serde(default)]
    pub user: Option<UserInfo>,
    #[serde(default)]
    pub sched_current: Option<ScheduleEntry>,
    #[serde(default)]
    pub sched_next: Vec<ScheduleEntry>,
    #[serde(rename = "request_line")]
    #[serde(default)]
    pub requests: Vec<RequestEntry>,
    #[serde(default)]
    pub listener_count: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AlbumArt {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<(u8, u8, u8)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_info_shape_with_null_request_song() {
        let json = r#"{
            "user": {"id": 1, "name": "Anonymous", "display_name": "Anonymous", "avatar": "/static/images4/user.svg"},
            "request_line": [{"username":"x","user_id":2,"song_id":null,"song":null,"position":1}],
            "sched_current": {
                "id": 10,
                "start_actual": 100,
                "end": 200,
                "type": "Election",
                "songs": [{"id": 44, "title": "Track", "length": 120, "entry_id": 555, "entry_votes": 3}]
            },
            "sched_next": [],
            "api_info": {"time": 1}
        }"#;

        let parsed: SyncData = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.user.unwrap().user_id, 1);
        assert_eq!(parsed.requests[0].id, 0);
        assert_eq!(parsed.requests[0].song.id, 0);
        let current = parsed.sched_current.unwrap();
        assert_eq!(current.sched_type, "Election");
        assert_eq!(current.end_actual, 200);
        assert_eq!(current.songs[0].entry_id, Some(555));
        assert_eq!(current.songs[0].votes, 3);
    }

    #[test]
    fn parses_live_station_shape() {
        let json = r#"{"id":1,"name":"Game","description":"OST radio","stream":"http://example/game.mp3","key":"game"}"#;
        let station: Station = serde_json::from_str(json).unwrap();
        assert_eq!(station.stream_url, "http://example/game.mp3");
        assert_eq!(station.genre, "game");
    }
}
