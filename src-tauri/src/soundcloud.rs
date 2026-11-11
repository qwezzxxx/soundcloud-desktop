use serde::{Deserialize, Serialize};



use serde_json::Value;
use std::collections::{HashMap, HashSet};



use std::fs;



use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;







const DEFAULT_CLIENT_ID: &str = "QhVEWrrJKvW4uSU5u5EoI0FTXYUGzy5k";
const PRECISE_WHISPER_MODEL: &str = "large-v3-turbo-q5_0";
const PRECISE_WHISPER_MODEL_SIZE: u64 = 574_041_195;



const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";







#[derive(Debug, Serialize, Deserialize, Clone)]



pub struct AppConfig {



    pub oauth_token: Option<String>,



    pub client_id: Option<String>,



    pub theme: Option<String>,



    pub volume: Option<u32>,



    pub user_id: Option<Value>,



    pub user_profile: Option<Value>,



    pub discord_rpc: Option<bool>,



    pub discord_client_id: Option<String>,



    pub discord_show_track: Option<bool>,



    pub discord_show_artist: Option<bool>,



    pub discord_show_cover: Option<bool>,



    pub discord_show_status: Option<bool>,



    #[serde(default)]



    pub cached_likes: Option<Vec<FormattedTrack>>,



    #[serde(default)]



    pub cached_mixed: Option<Value>,



    #[serde(default)]



    pub cached_trending: Option<Value>,



    #[serde(default)]



    pub accent_color: Option<String>,



    #[serde(default)]



    pub ui_radius: Option<String>,



    #[serde(default)]



    pub glow_intensity: Option<u32>,



    #[serde(default)]



    pub blur_enabled: Option<bool>,



    #[serde(default)]



    pub custom_bg: Option<String>,



    #[serde(default)]



    pub custom_bg_dim: Option<u32>,



    #[serde(default)]



    pub ui_opacity: Option<f64>,



    #[serde(default)]



    pub mixed_updated_at: Option<u64>,



    #[serde(default)]



    pub session: Option<Value>,



    #[serde(default)]



    pub last_tab: Option<String>,
    #[serde(default)]
    pub whisper_auto_transcribe: Option<bool>,
    #[serde(default = "default_whisper_precise")]
    pub whisper_precise: bool,



}







impl Default for AppConfig {



    fn default() -> Self {



        Self {



            oauth_token: Some("".to_string()),



            client_id: Some(DEFAULT_CLIENT_ID.to_string()),



            theme: Some("oled".to_string()),



            volume: Some(80),



            user_id: None,



            user_profile: None,



            discord_rpc: Some(true),



            discord_client_id: Some("802958833214423081".to_string()),



            discord_show_track: Some(true),



            discord_show_artist: Some(true),



            discord_show_cover: Some(true),



            discord_show_status: Some(true),



            cached_likes: None,



            cached_mixed: None,



            cached_trending: None,



            accent_color: Some("#ffffff".to_string()),



            ui_radius: Some("16px".to_string()),



            glow_intensity: Some(100),



            blur_enabled: Some(true),



            custom_bg: None,



            custom_bg_dim: Some(85),



            ui_opacity: Some(0.75),



            mixed_updated_at: None,



            session: None,



            last_tab: Some("tab-home".to_string()),
            whisper_auto_transcribe: Some(true),
            whisper_precise: default_whisper_precise(),



        }

    }

}







#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormattedTrack {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub liked: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub permalink_url: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub user_id: Option<Value>,
    #[serde(default)]
    pub user_permalink: Option<String>,
    #[serde(default)]
    pub user_avatar: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct LikesCache {
    account: Option<String>,
    tracks: Vec<FormattedTrack>,
    #[serde(default)]
    changes: HashMap<String, Vec<FormattedTrack>>,
}

impl LikesCache {
    fn track_id(id: &Value) -> String {
        id.as_str().map(str::to_owned).unwrap_or_else(|| id.to_string())
    }

    fn visible_tracks(&self, account: &str) -> Vec<FormattedTrack> {
        let tracks = if self.account.as_deref().is_none_or(|owner| owner == account) {
            self.tracks.as_slice()
        } else {
            &[]
        };
        let Some(changes) = self.changes.get(account) else {
            return tracks.to_vec();
        };
        let changed_ids: HashSet<_> = changes.iter().map(|track| Self::track_id(&track.id)).collect();
        changes.iter().filter(|track| track.liked).cloned().chain(
            tracks.iter().filter(|track| !changed_ids.contains(&Self::track_id(&track.id))).cloned()
        ).collect()
    }

    fn update_remote(&mut self, tracks: Vec<FormattedTrack>, account: &str, complete: bool, reconcile: bool) {
        if reconcile {
            if let Some(changes) = self.changes.get_mut(account) {
                let remote_ids: HashSet<_> = tracks.iter().map(|track| Self::track_id(&track.id)).collect();
                changes.retain(|track| {
                    let remote_liked = remote_ids.contains(&Self::track_id(&track.id));
                    if track.liked { !remote_liked } else { remote_liked || !complete }
                });
            }
        }
        self.account = Some(account.to_owned());
        self.tracks = tracks;
    }
}

#[cfg(test)]
mod likes_tests {
    use super::*;

    fn track(id: Value, liked: bool) -> FormattedTrack {
        serde_json::from_value(serde_json::json!({ "id": id, "title": "Track", "liked": liked })).unwrap()
    }

    #[test]
    fn remote_refresh_preserves_unconfirmed_changes() {
        let mut cache = LikesCache::default();
        cache.changes.insert("account".to_string(), vec![
            track(Value::from(1), true),
            track(Value::from("2"), false),
        ]);
        cache.update_remote(vec![track(Value::from(2), true)], "account", true, true);
        let visible = cache.visible_tracks("account");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, 1);
        assert!(cache.visible_tracks("other-account").is_empty());

        cache.update_remote(vec![track(Value::from("1"), true)], "account", false, true);
        assert_eq!(cache.changes["account"].len(), 1);
        cache.update_remote(vec![], "account", true, false);
        assert_eq!(cache.changes["account"].len(), 1);
        cache.update_remote(vec![], "account", true, true);
        assert!(cache.changes["account"].is_empty());
    }

    #[tokio::test]
    async fn guest_like_and_unlike_survive_reload_with_a_legacy_cache() {
        let folder = std::env::temp_dir().join(format!("onyx-likes-test-{}", std::process::id()));
        fs::create_dir(&folder).unwrap();
        let config_path = folder.join("config.json");
        let legacy_path = folder.join("likes.json");
        let track = track(Value::from(1), true);
        fs::write(&legacy_path, serde_json::to_vec(&[&track]).unwrap()).unwrap();

        let client = SoundCloudClient::new(&config_path);
        assert_eq!(client.load_local_likes().len(), 1);
        assert_eq!(client.toggle_like(track.clone(), false).await.unwrap(), None);
        let restarted = SoundCloudClient::new(&config_path);
        assert!(restarted.load_local_likes().is_empty());
        assert_eq!(restarted.toggle_like(track, true).await.unwrap(), None);
        assert_eq!(SoundCloudClient::new(&config_path).load_local_likes().len(), 1);

        fs::remove_file(legacy_path).unwrap();
        fs::remove_file(folder.join("likes-state.json")).unwrap();
        fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn expired_jwt_token_is_detected_and_user_extracted() {
        // Sample token with exp=1000 (way in the past) and sub="soundcloud:users:1234567"
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6ImF0K0pXVCJ9";
        let payload = "eyJzdWIiOiJzb3VuZGNsb3VkOnVzZXJzOjEyMzQ1NjciLCJleHAiOjEwMDB9";
        let fake_jwt = format!("{}.{}.fake_signature", header, payload);

        assert!(SoundCloudClient::is_token_expired(&fake_jwt));
        assert_eq!(SoundCloudClient::extract_user_id_from_token(&fake_jwt).as_deref(), Some("1234567"));

        // Non-JWT token is not considered expired by format
        assert!(!SoundCloudClient::is_token_expired("2-29384-12345"));
        assert_eq!(SoundCloudClient::extract_user_id_from_token("2-29384-12345"), None);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormattedArtist {
    pub id: Value,
    pub username: String,
    pub permalink: String,
    pub avatar_url: String,
    pub banner_url: Option<String>,
    pub followers_count: u64,
    pub track_count: u64,
    pub description: Option<String>,
    pub verified: bool,
    pub city: Option<String>,
    pub country: Option<String>,
    pub permalink_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormattedAlbum {
    pub id: Value,
    pub title: String,
    pub artist: String,
    pub cover: String,
    pub track_count: usize,
    pub release_year: Option<String>,
    pub permalink_url: Option<String>,
}







// Cache of YouTube search results: "artist|title|duration_ms" -> (video_id, timestamp).
// The search step alone costs ~3s (yt-dlp process + network), so remember it per session.
static YT_SEARCH_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>,
> = std::sync::OnceLock::new();

pub struct SoundCloudClient {
    http: reqwest::Client,
    config_path: PathBuf,
    like_rate_limited_until: std::sync::Mutex<Option<std::time::Instant>>,
    likes_revision: std::sync::Mutex<u64>,
    like_requests: tokio::sync::Mutex<()>,
    likes_refresh: tokio::sync::Mutex<()>,
    whisper_job: tokio::sync::Mutex<()>,
    config_write: std::sync::Mutex<()>,
}







impl SoundCloudClient {



    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn is_like_rate_limited(&self) -> bool {
        if let Ok(guard) = self.like_rate_limited_until.lock() {
            if let Some(until) = *guard {
                if std::time::Instant::now() < until {
                    return true;
                }
            }
        }
        false
    }

    pub fn set_like_rate_limited(&self, duration: std::time::Duration) {
        if let Ok(mut guard) = self.like_rate_limited_until.lock() {
            *guard = Some(std::time::Instant::now() + duration);
        }
    }







    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {



        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(std::time::Duration::from_secs(6))
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap_or_default();



        Self {
            http,
            config_path: config_path.as_ref().to_path_buf(),
            like_rate_limited_until: std::sync::Mutex::new(None),
            likes_revision: std::sync::Mutex::new(0),
            like_requests: tokio::sync::Mutex::new(()),
            likes_refresh: tokio::sync::Mutex::new(()),
            whisper_job: tokio::sync::Mutex::new(()),
            config_write: std::sync::Mutex::new(()),
        }



    }







    pub fn likes_path(&self) -> PathBuf {
        if let Some(parent) = self.config_path.parent() {
            parent.join("likes.json")
        } else {
            PathBuf::from("likes.json")
        }
    }

    pub fn load_local_likes(&self) -> Vec<FormattedTrack> {
        let _guard = self.likes_revision.lock().unwrap();
        let cfg = self.load_config();
        self.load_likes_cache(&cfg).visible_tracks(&Self::likes_account(&cfg))
    }

    fn likes_account(cfg: &AppConfig) -> String {
        cfg.user_id.as_ref().filter(|id| !id.is_null())
            .or_else(|| cfg.user_profile.as_ref().and_then(|profile| profile.get("id")))
            .map(LikesCache::track_id).unwrap_or_default()
    }

    fn load_likes_cache(&self, cfg: &AppConfig) -> LikesCache {
        if let Ok(content) = fs::read_to_string(self.likes_path().with_file_name("likes-state.json")) {
            if let Ok(cache) = serde_json::from_str::<LikesCache>(&content) {
                return cache;
            }
        }
        if let Ok(content) = fs::read_to_string(self.likes_path()) {
            if let Ok(tracks) = serde_json::from_str::<Vec<FormattedTrack>>(&content) {
                return LikesCache { tracks, ..LikesCache::default() };
            }
        }
        LikesCache { tracks: cfg.cached_likes.clone().unwrap_or_default(), ..LikesCache::default() }
    }

    fn save_likes_cache(&self, cache: &LikesCache) -> Result<(), String> {
        let l_path = self.likes_path().with_file_name("likes-state.json");
        let temp_path = l_path.with_extension("json.tmp");
        let json = serde_json::to_vec(cache).map_err(|e| e.to_string())?;
        fs::write(&temp_path, json).and_then(|_| fs::rename(&temp_path, &l_path))
            .map_err(|e| format!("Не удалось сохранить лайки в ONYX: {e}"))
    }

    pub fn load_config(&self) -> AppConfig {



        if let Ok(content) = fs::read_to_string(&self.config_path) {



            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {



                return cfg;



            }



        }



        AppConfig::default()



    }







    pub fn save_config(&self, cfg: &AppConfig) -> Result<(), String> {
        let _guard = self.config_write.lock().map_err(|e| e.to_string())?;
        let existing = self.load_config();
        let mut to_save = cfg.clone();

        // General settings update: preserve credentials and session
        to_save.oauth_token = existing.oauth_token;
        if to_save.user_id.is_none() && existing.user_id.is_some() {
            to_save.user_id = existing.user_id;
        }
        if to_save.user_profile.is_none() && existing.user_profile.is_some() {
            to_save.user_profile = existing.user_profile;
        }
        if to_save.cached_likes.as_ref().map(|l| l.is_empty()).unwrap_or(true) && existing.cached_likes.as_ref().map(|l| !l.is_empty()).unwrap_or(false) {
            to_save.cached_likes = existing.cached_likes;
        }
        if to_save.cached_mixed.is_none() && existing.cached_mixed.is_some() {
            to_save.cached_mixed = existing.cached_mixed;
        }
        if to_save.cached_trending.is_none() && existing.cached_trending.is_some() {
            to_save.cached_trending = existing.cached_trending;
        }
        if to_save.session.is_none() && existing.session.is_some() {
            to_save.session = existing.session;
        }
        if to_save.last_tab.is_none() && existing.last_tab.is_some() {
            to_save.last_tab = existing.last_tab;
        }

        self.write_config_file(&to_save)
    }

    pub fn save_raw_config(&self, cfg: &AppConfig) -> Result<(), String> {
        let _guard = self.config_write.lock().map_err(|e| e.to_string())?;
        self.write_config_file(cfg)
    }

    fn write_config_file(&self, cfg: &AppConfig) -> Result<(), String> {
        let directory = self.config_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(staged.as_file_mut(), cfg).map_err(|e| e.to_string())?;
        staged.as_file().sync_all().map_err(|e| e.to_string())?;
        staged.persist(&self.config_path).map_err(|e| e.error.to_string())?;
        Ok(())
    }







    fn decode_base64url(s: &str) -> Option<Vec<u8>> {
        let mut table = [255u8; 256];
        for (i, &b) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".iter().enumerate() {
            table[b as usize] = i as u8;
        }
        table[b'+' as usize] = 62;
        table[b'/' as usize] = 63;

        let clean = s.trim_end_matches('=');
        let mut bytes = Vec::with_capacity(clean.len() * 3 / 4);
        let mut buf = 0u32;
        let mut bits = 0;
        for &b in clean.as_bytes() {
            let val = table[b as usize];
            if val == 255 { return None; }
            buf = (buf << 6) | (val as u32);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                bytes.push((buf >> bits) as u8);
            }
        }
        Some(bytes)
    }

    pub fn is_token_expired(token: &str) -> bool {
        let clean = token.trim().strip_prefix("OAuth ").or_else(|| token.trim().strip_prefix("Bearer ")).unwrap_or(token.trim());
        let mut parts = clean.split('.');
        let _header = parts.next();
        if let Some(payload_b64) = parts.next() {
            if parts.next().is_some() {
                if let Some(bytes) = Self::decode_base64url(payload_b64) {
                    if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                        if let Some(exp) = val.get("exp").and_then(|v| v.as_i64()) {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0);
                            return now >= exp;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn extract_user_id_from_token(token: &str) -> Option<String> {
        let clean = token.trim().strip_prefix("OAuth ").or_else(|| token.trim().strip_prefix("Bearer ")).unwrap_or(token.trim());
        let mut parts = clean.split('.');
        let _header = parts.next();
        let payload_b64 = parts.next()?;
        if parts.next().is_none() { return None; }
        let bytes = Self::decode_base64url(payload_b64)?;
        let val: Value = serde_json::from_slice(&bytes).ok()?;
        let sub = val.get("sub").and_then(|v| v.as_str())?;
        let uid = sub.rsplit(':').next()?;
        if !uid.is_empty() && uid.chars().all(|c| c.is_ascii_digit()) {
            Some(uid.to_string())
        } else {
            None
        }
    }

    fn auth_header(&self, cfg: &AppConfig) -> Option<String> {
        cfg.oauth_token.as_ref().and_then(|t| {
            let trimmed = t.trim().trim_matches(&['"', '\''][..]).trim();
            if trimmed.is_empty() || Self::is_token_expired(trimmed) {
                None
            } else if trimmed.starts_with("OAuth ") || trimmed.starts_with("Bearer ") {
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let scheme = parts[0];
                    let raw_token = parts[1].trim().trim_matches(&['"', '\''][..]).trim();
                    Some(format!("{} {}", scheme, raw_token))
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                Some(format!("OAuth {}", trimmed))
            }
        })
    }

    pub async fn ensure_active_user(&self) -> Result<(i64, String), String> {
        let cfg = self.load_config();
        let token = self.auth_header(&cfg).ok_or("Not logged in")?;
        let client_id = self.get_client_id().await;

        if let Some(ref uid_val) = cfg.user_id {
            if let Some(n) = uid_val.as_i64() {
                if n > 0 && n != 1000000 {
                    return Ok((n, token));
                }
            } else if let Some(s) = uid_val.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    if n > 0 && n != 1000000 {
                        return Ok((n, token));
                    }
                }
            }
        }

        let me_url = format!("https://api-v2.soundcloud.com/me?client_id={}", client_id);
        let res = self.http.get(&me_url)
            .header("Authorization", &token)
            .header("User-Agent", USER_AGENT)
            .header("Origin", "https://soundcloud.com")
            .header("Referer", "https://soundcloud.com/")
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .send()
            .await
            .map_err(|e| format!("Failed to reach SoundCloud /me: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("SoundCloud /me returned HTTP {}", res.status()));
        }

        let user_val: Value = res.json().await.map_err(|e| format!("Failed to parse /me: {}", e))?;
        let numeric_id = user_val.get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "SoundCloud /me response does not contain numeric user id".to_string())?;

        let mut upd = self.load_config();
        upd.user_id = Some(Value::from(numeric_id));
        upd.user_profile = Some(user_val);
        let _ = self.save_config(&upd);

        Ok((numeric_id, token))
    }

    pub async fn fetch_remote_liked_ids(
        &self,
        user_id: i64,
        token: &str,
        client_id: &str,
    ) -> Result<std::collections::HashSet<String>, String> {
        let mut ids = std::collections::HashSet::new();
        let mut next_url = Some(format!(
            "https://api-v2.soundcloud.com/users/{}/track_likes?client_id={}&limit=50&linked_partitioning=1",
            user_id, client_id
        ));
        let mut pages = 0;

        while let Some(url) = next_url {
            pages += 1;
            if pages > 20 || ids.len() >= 1000 {
                break;
            }

            let req = self.http.get(&url)
                .header("Authorization", token)
                .header("User-Agent", USER_AGENT)
                .header("Accept", "application/json")
                .header("Origin", "https://soundcloud.com")
                .header("Referer", "https://soundcloud.com/");

            let res = req.send().await.map_err(|e| e.to_string())?;
            let status = res.status().as_u16();
            if status == 429 {
                return Err("SoundCloud временно ограничил запросы (429 Too Many Requests).".to_string());
            }
            if !res.status().is_success() {
                break;
            }

            let body: Value = res.json().await.map_err(|e| e.to_string())?;
            let collection = body.get("collection").and_then(|v| v.as_array());
            if let Some(items) = collection {
                if items.is_empty() {
                    break;
                }
                for item in items {
                    let track_obj = if item.get("track").is_some() {
                        item.get("track").unwrap()
                    } else {
                        item
                    };
                    if let Some(id_val) = track_obj.get("id") {
                        let id_str = if let Some(n) = id_val.as_i64() {
                            n.to_string()
                        } else if let Some(s) = id_val.as_str() {
                            s.to_string()
                        } else {
                            id_val.to_string().trim_matches(&['"', '\''][..]).to_string()
                        };
                        if !id_str.is_empty() && id_str != "null" {
                            ids.insert(id_str);
                        }
                    }
                }
            } else {
                break;
            }

            let next = body.get("next_href").and_then(Value::as_str);
            if next.is_some_and(|url| !is_soundcloud_api_url(url)) {
                return Err("Некорректная страница лайков SoundCloud".to_string());
            }
            next_url = next.map(|s| {
                if !s.contains("client_id=") {
                    let sep = if s.contains('?') { "&" } else { "?" };
                    format!("{}{}client_id={}", s, sep, client_id)
                } else {
                    s.to_string()
                }
            });
        }

        Ok(ids)
    }

    pub async fn push_local_likes_to_soundcloud(&self) -> Result<usize, String> {
        if self.is_like_rate_limited() {
            return Err("SoundCloud кулдаун 429 активен. Автоматическая отправка лайков приостановлена.".to_string());
        }
        let (user_id, token) = match self.ensure_active_user().await {
            Ok(pair) => pair,
            Err(_) => return Ok(0),
        };
        let client_id = self.get_client_id().await;
        let local_tracks = self.load_local_likes();
        if local_tracks.is_empty() {
            return Ok(0);
        }

        // Fetch currently liked tracks directly from SoundCloud API v2 (WITHOUT local cache merge)
        let remote_ids = match self.fetch_remote_liked_ids(user_id, &token, &client_id).await {
            Ok(ids) => ids,
            Err(e) => {
                // If remote fetch failed (e.g. rate limited or offline), abort safely without spamming
                return Err(e);
            }
        };

        let mut pushed = 0;
        for track in local_tracks {
            let id_str = if let Some(n) = track.id.as_i64() {
                n.to_string()
            } else if let Some(s) = track.id.as_str() {
                s.to_string()
            } else {
                track.id.to_string().trim_matches(&['"', '\''][..]).to_string()
            };
            if id_str.is_empty() || id_str == "null" {
                continue;
            }

            // Skip tracks that are ALREADY liked on SoundCloud
            if remote_ids.contains(&id_str) {
                continue;
            }

            let put_url = format!(
                "https://api-v2.soundcloud.com/users/{}/track_likes/{}?client_id={}",
                user_id, id_str, client_id
            );
            let res = self.http.put(&put_url)
                .header("Authorization", &token)
                .header("Origin", "https://soundcloud.com")
                .header("Referer", "https://soundcloud.com/")
                .header("User-Agent", USER_AGENT)
                .header("Accept", "application/json, text/javascript, */*; q=0.01")
                .header("Content-Length", "0")
                .body("")
                .send()
                .await;

            if let Ok(r) = res {
                let st = r.status().as_u16();
                if st == 200 || st == 201 || st == 204 {
                    pushed += 1;
                } else if st == 429 {
                    self.set_like_rate_limited(std::time::Duration::from_secs(1800));
                    return Err("SoundCloud временно заблокировал добавление лайков (рейт-лимит 429). Подождите 15-30 минут.".to_string());
                }
            }

            // Polite delay between requests to never trigger rate limits
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Ok(pushed)
    }

    pub async fn refresh_dynamic_client_id(&self) -> Option<String> {



        let res = self.http.get("https://soundcloud.com")



            .header("User-Agent", USER_AGENT)



            .send().await.ok()?;



        let html = res.text().await.ok()?;



        let key = r#""hydratable":"apiClient""#;



        let idx = html.find(key)?;



        let slice = &html[idx..];



        let id_marker = r#""id":""#;



        let id_start = slice.find(id_marker)? + id_marker.len();



        let rest = &slice[id_start..];



        let id_end = rest.find('"')?;



        let id = rest[..id_end].to_string();



        if id.len() > 10 {



            let mut cfg = self.load_config();



            cfg.client_id = Some(id.clone());



            let _ = self.save_config(&cfg);



            Some(id)



        } else {



            None



        }



    }







    pub async fn get_client_id(&self) -> String {



        let cfg = self.load_config();



        if let Some(ref cid) = cfg.client_id {



            if cid.len() > 10 && cid != "dbe62de186a100fbdf5bcab837fcfbc2" && cid != "3GKD969NJWD1eB6gvQJN0GnueYbcWszv" {



                return cid.clone();



            }



        }



        if let Some(dyn_id) = self.refresh_dynamic_client_id().await {



            return dyn_id;



        }



        cfg.client_id.unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string())



    }







    pub async fn get_profile(&self) -> Result<Value, String> {



        let cfg = self.load_config();



        let token_opt = self.auth_header(&cfg);



        let client_id = self.get_client_id().await;







        let (url, has_auth) = if let Some(ref _t) = token_opt {
            (format!("https://api-v2.soundcloud.com/me?client_id={}", client_id), true)
        } else if let Some(ref uid) = cfg.user_id {
            let uid_str = if let Some(n) = uid.as_i64() {
                n.to_string()
            } else if let Some(s) = uid.as_str() {
                s.to_string()
            } else {
                uid.to_string().trim_matches('"').to_string()
            };

            if uid_str.is_empty() {
                return Err("No user configured".to_string());
            }

            (format!("https://api-v2.soundcloud.com/users/{}?client_id={}", uid_str, client_id), false)
        } else if let Some(ref raw_tok) = cfg.oauth_token {
            if let Some(uid_str) = Self::extract_user_id_from_token(raw_tok) {
                (format!("https://api-v2.soundcloud.com/users/{}?client_id={}", uid_str, client_id), false)
            } else {
                return Err("No oauth token or user configured".to_string());
            }
        } else {
            return Err("No oauth token or user configured".to_string());
        };

        let mut req = self
            .http
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json");

        if has_auth {
            if let Some(tok) = token_opt {
                req = req.header("Authorization", tok);
            }
        }

        let mut res = req
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status() == reqwest::StatusCode::UNAUTHORIZED && has_auth {
            // /me rejected with 401: fall back to public /users/{id}
            let fallback_uid = cfg.user_id.as_ref().and_then(|u| {
                if let Some(n) = u.as_i64() { Some(n.to_string()) }
                else if let Some(s) = u.as_str() { Some(s.to_string()) }
                else { None }
            }).or_else(|| cfg.oauth_token.as_ref().and_then(|t| Self::extract_user_id_from_token(t)));

            if let Some(uid_str) = fallback_uid {
                let fallback_url = format!("https://api-v2.soundcloud.com/users/{}?client_id={}", uid_str, client_id);
                if let Ok(r) = self.http.get(&fallback_url).header("User-Agent", USER_AGENT).header("Accept", "application/json").send().await {
                    if r.status().is_success() {
                        res = r;
                    }
                }
            }
        }

        if res.status().is_success() {
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            let mut updated_cfg = cfg.clone();
            updated_cfg.user_id = data.get("id").cloned();
            updated_cfg.user_profile = Some(data.clone());
            let _ = self.save_config(&updated_cfg);
            Ok(data)
        } else if let Some(ref prof) = cfg.user_profile {
            Ok(prof.clone())
        } else {
            Err(format!("Profile request failed: HTTP {}", res.status()))
        }



    }







    pub fn format_track(t: &Value, is_liked: bool) -> FormattedTrack {



        let id = t.get("id").cloned().unwrap_or(Value::Null);



        let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();



        let uploader = t.get("user")
            .and_then(|u| u.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist")
            .to_string();

        let pub_artist = t.get("publisher_metadata")
            .and_then(|p| p.get("artist"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let artist = pub_artist.unwrap_or(uploader);







        let mut artwork = t.get("artwork_url")



            .and_then(|v| v.as_str())



            .or_else(|| t.get("user").and_then(|u| u.get("avatar_url")).and_then(|v| v.as_str()))



            .unwrap_or("")



            .to_string();







        if artwork.contains("large.jpg") {



            artwork = artwork.replace("large.jpg", "t500x500.jpg");



        }



        if artwork.is_empty() {



            artwork = "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=300".to_string();



        }







        let duration_ms = t.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);



        let mins = duration_ms / 60000;



        let secs = (duration_ms % 60000) / 1000;



        let duration = format!("{}:{:02}", mins, secs);







        let liked = is_liked || t.get("user_favorite").and_then(|v| v.as_bool()).unwrap_or(false);



        let description = t.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());



        let genre = t.get("genre").and_then(|v| v.as_str()).map(|s| s.to_string());



        let permalink_url = t.get("permalink_url").and_then(|v| v.as_str()).map(|s| s.to_string());







        let user_id = t.get("user").and_then(|u| u.get("id")).cloned();
        let user_permalink = t.get("user").and_then(|u| u.get("permalink")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let user_avatar = t.get("user").and_then(|u| u.get("avatar_url")).and_then(|v| v.as_str()).map(|s| s.to_string());

        FormattedTrack {
            id,
            title,
            artist,
            duration,
            duration_ms,
            cover: artwork,
            liked,
            description,
            genre,
            permalink_url,
            provider: Some("soundcloud".to_string()),
            user_id,
            user_permalink,
            user_avatar,
        }



    }


    pub fn lyrics_cache_dir(&self) -> PathBuf {
        let dir = self.config_path.parent().unwrap_or(Path::new(".")).join("lyrics_cache");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn find_whisper(precise: bool) -> Option<PathBuf> {
        if precise {
            let relative = Path::new("whisper").join("cuda").join("whisper-cli.exe");
            let preferred = std::env::current_exe().ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join(&relative)))
                .filter(|path| path.is_file());
            let local = Path::new(r"F:\Soundcloud").join(relative);
            if let Some(path) = preferred.or_else(|| local.is_file().then_some(local)) {
                return Some(path);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                for name in &["whisper-cli.exe", "whisper.exe"] {
                    let cand = dir.join(name);
                    if cand.exists() { return Some(cand); }
                    let cand_sub = dir.join("whisper").join(name);
                    if cand_sub.exists() { return Some(cand_sub); }
                }
            }
        }
        for name in &["whisper-cli.exe", "whisper.exe"] {
            let cand_f = PathBuf::from(r"F:\Soundcloud").join(name);
            if cand_f.exists() { return Some(cand_f); }
            let cand_f_sub = PathBuf::from(r"F:\Soundcloud\whisper").join(name);
            if cand_f_sub.exists() { return Some(cand_f_sub); }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(appdata).join("com.onyx.player").join("whisper");
            for name in &["whisper-cli.exe", "whisper.exe"] {
                let cand = base.join(name);
                if cand.exists() { return Some(cand); }
            }
        }
        None
    }

    pub fn find_whisper_model() -> Option<PathBuf> {
        let model_names = ["ggml-tiny.bin", "ggml-base.bin", "ggml-small.bin"];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                for name in &model_names {
                    let cand = dir.join(name);
                    if cand.exists() { return Some(cand); }
                    let cand_m = dir.join("models").join(name);
                    if cand_m.exists() { return Some(cand_m); }
                    let cand_w = dir.join("whisper").join(name);
                    if cand_w.exists() { return Some(cand_w); }
                }
            }
        }
        for name in &model_names {
            let cand_f = PathBuf::from(r"F:\Soundcloud\models").join(name);
            if cand_f.exists() { return Some(cand_f); }
            let cand_f_root = PathBuf::from(r"F:\Soundcloud").join(name);
            if cand_f_root.exists() { return Some(cand_f_root); }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(appdata).join("com.onyx.player").join("models");
            for name in &model_names {
                let cand = base.join(name);
                if cand.exists() { return Some(cand); }
            }
        }
        None
    }

    pub fn find_ffmpeg() -> Option<PathBuf> {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let cand = dir.join("ffmpeg.exe");
                if cand.exists() { return Some(cand); }
                let cand_sub = dir.join("whisper").join("ffmpeg.exe");
                if cand_sub.exists() { return Some(cand_sub); }
            }
        }
        let cand_f = PathBuf::from(r"F:\Soundcloud\whisper\ffmpeg.exe");
        if cand_f.exists() { return Some(cand_f); }
        let cand_f2 = PathBuf::from(r"F:\Soundcloud\ffmpeg.exe");
        if cand_f2.exists() { return Some(cand_f2); }

        if let Ok(p) = std::env::var("LOCALAPPDATA") {
            let win_get = PathBuf::from(p).join("Microsoft").join("WinGet").join("Packages");
            if win_get.exists() {
                if let Ok(entries) = std::fs::read_dir(&win_get) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            if fname.contains("FFmpeg") {
                                let c1 = path.join("ffmpeg.exe");
                                if c1.exists() { return Some(c1); }
                                let c2 = path.join("bin").join("ffmpeg.exe");
                                if c2.exists() { return Some(c2); }
                                if let Ok(subs) = std::fs::read_dir(&path) {
                                    for sub in subs.flatten() {
                                        let c3 = sub.path().join("bin").join("ffmpeg.exe");
                                        if c3.exists() { return Some(c3); }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn ensure_whisper_model(&self) -> Result<PathBuf, String> {
        if let Some(m) = Self::find_whisper_model() {
            return Ok(m);
        }
        self.ensure_whisper_model_named("tiny").await
    }

    async fn ensure_whisper_model_named(&self, model_name: &str) -> Result<PathBuf, String> {
        let filename = format!("ggml-{}.bin", model_name);
        let model_size = (model_name == PRECISE_WHISPER_MODEL).then_some(PRECISE_WHISPER_MODEL_SIZE);
        let base_dir = self.config_path.parent().unwrap_or(Path::new(".")).join("models");
        let target = base_dir.join(&filename);
        let mut candidates = Vec::new();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join(&filename));
                candidates.push(dir.join("models").join(&filename));
                candidates.push(dir.join("whisper").join(&filename));
            }
        }
        candidates.push(PathBuf::from(r"F:\Soundcloud\models").join(&filename));
        candidates.push(PathBuf::from(r"F:\Soundcloud").join(&filename));
        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(PathBuf::from(appdata).join("com.onyx.player").join("models").join(&filename));
        }
        candidates.push(target.clone());

        if let Some(path) = candidates.into_iter().find(|path| {
            path.metadata().is_ok_and(|meta| meta.is_file() && model_size.unwrap_or(meta.len()) == meta.len())
        }) {
            return Ok(path);
        }

        fs::create_dir_all(&base_dir).map_err(|e| format!("Не удалось создать папку моделей Whisper: {}", e))?;
        let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}?download=true", filename);
        let mut response = self.http.get(url)
            .timeout(std::time::Duration::from_secs(1800))
            .send().await.and_then(reqwest::Response::error_for_status)
            .map_err(|e| format!("Не удалось скачать модель Whisper {}: {}", model_name, e))?;
        let expected_size = model_size.or(response.content_length());
        let (file, staged_path) = tempfile::NamedTempFile::new_in(&base_dir)
            .map_err(|e| format!("Не удалось создать временный файл модели: {}", e))?.into_parts();
        let mut file = tokio::fs::File::from_std(file);
        let mut written = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(|e| format!("Загрузка Whisper прервана: {}", e))? {
            file.write_all(&chunk).await.map_err(|e| format!("Не удалось записать модель Whisper: {}", e))?;
            written += chunk.len() as u64;
        }
        if written == 0 || expected_size.is_some_and(|size| size != written) {
            return Err(format!("Модель Whisper {} загружена не полностью", model_name));
        }
        file.flush().await.map_err(|e| format!("Не удалось завершить запись модели Whisper: {}", e))?;
        drop(file);
        staged_path.persist(&target).map_err(|e| format!("Не удалось сохранить модель Whisper: {}", e))?;
        Ok(target)
    }

    fn whisper_has_lyric_words(text: &str) -> bool {
        let label = text.trim().trim_matches(|c: char| c.is_whitespace() || "[]()♪♫«»\"'".contains(c));
        if !label.chars().any(char::is_alphanumeric) {
            return false;
        }
        let lower = label.to_lowercase();
        let is_noise = matches!(
            lower.as_str(),
            "music" | "instrumental" | "background music" | "музыка" | "инструментал" | "апплодисменты" | "applause"
        ) || lower.contains("субтитр")
          || lower.contains("subtitl")
          || lower.contains("dimator")
          || lower.contains("дима торжок")
          || lower.contains("диматор")
          || lower.contains("корректор")
          || lower.contains("подогнал")
          || lower.contains("симон")
          || lower.contains("семкин")
          || lower.contains("семецкая")
          || lower.contains("синецкая")
          || lower.contains("сухиашвили")
          || lower.contains("кулакова")
          || lower.contains("егорова")
          || lower.contains("community.org")
          || lower.contains("opensubtitles")
          || lower.contains("amara.org")
          || lower.contains("редактор")
          || lower.contains("перевод")
          || lower.contains("подписывайтесь")
          || lower.contains("ставьте лайк")
          || lower.contains("приятного просмотра")
          || lower.contains("thank you for watching")
          || lower.contains("thanks for watching");
        !is_noise
    }

    fn whisper_line_text(line: &str) -> String {
        let text = line.split_once(']').map(|(_, text)| text).unwrap_or(line);
        let mut parts = text.split('<');
        let mut plain = parts.next().unwrap_or_default().to_string();
        for part in parts {
            plain.push_str(part.split_once('>').map(|(_, word)| word).unwrap_or(part));
        }
        plain
    }

    fn clean_whisper_lrc(lrc: &str) -> String {
        lrc.lines()
            .filter(|line| Self::whisper_has_lyric_words(&Self::whisper_line_text(line)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn whisper_output_is_low_quality(lrc: &str, json_path: &Path, precise: bool) -> bool {
        let lines = lrc.lines()
            .map(Self::whisper_line_text)
            .filter(|line| Self::whisper_has_lyric_words(line))
            .collect::<Vec<_>>();
        let normalized = lines.iter()
            .map(|line| line.split(|c: char| !c.is_alphanumeric())
                .filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ").to_lowercase())
            .collect::<Vec<_>>();
        if normalized.iter().any(|line| line.starts_with("this is a production of the u s department of"))
            || (2..=3).any(|period| normalized.len() >= period * 4
                && normalized.iter().skip(period).zip(&normalized).all(|(line, previous)| line == previous))
        {
            return true;
        }
        let words = lines.iter()
            .flat_map(|line| line.split_whitespace())
            .filter(|word| word.chars().any(char::is_alphanumeric))
            .count();
        if words < 3 {
            return true;
        }
        if precise {
            // Long runs of one line can occur when music is mistaken for vocals.
            let mut previous = "";
            let mut run = 0;
            let mut longest = 0;
            let mut count = 0;
            for line in &normalized {
                run = if line == previous { run + 1 } else { 1 };
                longest = longest.max(run);
                previous = line;
                count += 1;
            }
            if (longest == count && longest >= 3)
                || (longest >= 8 && longest * 2 >= count)
                || (longest >= 4 && longest * 4 >= count * 3)
            {
                return true;
            }
        }

        let Ok(json_text) = fs::read_to_string(json_path) else { return false; };
        let Ok(data) = serde_json::from_str::<Value>(&json_text) else { return false; };
        let Some(segments) = data.get("transcription").and_then(Value::as_array) else { return false; };
        let mut scored = 0usize;
        let mut weak = 0usize;
        for segment in segments {
            let Some(text) = segment.get("text").and_then(Value::as_str) else { continue; };
            if !Self::whisper_has_lyric_words(text) { continue; }
            let token_probs = segment.get("tokens").and_then(Value::as_array)
                .into_iter().flatten()
                .filter_map(|token| token.get("p").and_then(Value::as_f64))
                .collect::<Vec<_>>();
            if !token_probs.is_empty() {
                scored += 1;
                let mean_probability = token_probs.iter().sum::<f64>() / token_probs.len() as f64;
                if mean_probability < 0.12 { weak += 1; }
            }
        }
        scored > 0 && weak * 2 >= scored
    }

    pub async fn transcribe_audio_stream(
        &self,
        track_id: &str,
        stream_url: &str,
        precise: bool,
        title: Option<&str>,
        artist: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<Value, String> {
        let clean_id: String = track_id.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
        if clean_id.is_empty() {
            return Err("Invalid track id".to_string());
        }

        // Serialize decoding and check the cache again for queued requests.
        let _job = self.whisper_job.lock().await;
        let cache_dir = self.lyrics_cache_dir();
        let suffix_v3 = if precise { "large-v3-turbo-q5.v3" } else { "v3" };
        let suffix_v2 = if precise { "large-v3-turbo-q5.v2" } else { "v2" };
        let cache_file_v3 = cache_dir.join(format!("{}.{}.lrc", clean_id, suffix_v3));
        let cache_file_v2 = cache_dir.join(format!("{}.{}.lrc", clean_id, suffix_v2));

        let cached_content = fs::read_to_string(&cache_file_v3).or_else(|_| fs::read_to_string(&cache_file_v2));
        if let Ok(content) = cached_content {
            let content = Self::clean_whisper_lrc(&content);
            let plain = Self::whisper_lyrics_result(content, precise);
            let plain_text = plain.get("plainLyrics").and_then(Value::as_str).unwrap_or_default();
            if !Self::whisper_output_is_low_quality(plain_text, Path::new(""), true) {
                return Ok(plain);
            }
        }
        if !is_transcription_stream_url(stream_url) {
            return Err("Недопустимый адрес аудиопотока".to_string());
        }

        let whisper_exe = Self::find_whisper(precise).ok_or_else(|| "Движок Whisper не найден (положите whisper-cli.exe в папку whisper)".to_string())?;
        let ffmpeg_exe = Self::find_ffmpeg().ok_or_else(|| "FFmpeg не найден для аудио-конвертации".to_string())?;

        let temp_dir = tempfile::Builder::new().prefix("onyx-whisper-").tempdir()
            .map_err(|e| format!("Не удалось создать временную папку Whisper: {}", e))?;
        let temp_wav = temp_dir.path().join("audio.wav");
        let temp_out_base = temp_dir.path().join("lyrics");
        let temp_lrc = temp_out_base.with_extension("lrc");

        // 2. Convert stream into 16kHz mono WAV using ffmpeg with vocal optimization filter
        let mut ff_cmd = tokio::process::Command::new(&ffmpeg_exe);
        ff_cmd.kill_on_drop(true).args([
            "-nostdin", "-hide_banner", "-loglevel", "error", "-y",
            "-protocol_whitelist", "http,https,tcp,tls,crypto",
            "-headers", "Origin: https://soundcloud.com\r\nReferer: https://soundcloud.com/\r\n",
            "-i", stream_url,
        ]);
        if precise {
            ff_cmd.arg("-xerror");
        } else {
            ff_cmd.args(["-t", "360"]);
        }
        // Vocal optimization:
        // highpass=f=120: cut off deep 808 sub-bass, kick punch rumble
        // lowpass=f=7500: cut off cymbal/hi-hat sizzle and extreme high frequencies
        // acompressor: normalize dynamics so quiet verses and loud hooks are balanced
        ff_cmd.args([
            "-af", "highpass=f=120,lowpass=f=7500,acompressor=threshold=-18dB:ratio=3:attack=5:release=50",
            "-vn", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"
        ]).arg(&temp_wav);
        #[cfg(windows)]
        ff_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let audio_timeout = if precise { 180 } else { 35 };
        let ff_status = tokio::time::timeout(std::time::Duration::from_secs(audio_timeout), ff_cmd.status()).await
            .map_err(|_| format!("Таймаут скачивания аудио для распознавания ({} сек)", audio_timeout))?
            .map_err(|e| format!("Ошибка запуска ffmpeg: {}", e))?;

        if !ff_status.success() || !temp_wav.exists() {
            return Err("Не удалось подготовить аудиофайл для Whisper".to_string());
        }

        let model_path = if precise {
            self.ensure_whisper_model_named(PRECISE_WHISPER_MODEL).await?
        } else {
            self.ensure_whisper_model().await?
        };

        // Construct bilingual / context prompt
        let prompt_arg: Option<String> = {
            if let Some(p) = prompt.filter(|p| !p.trim().is_empty()) {
                let snippet: String = p.trim().chars().take(350).collect();
                Some(snippet)
            } else {
                let t = title.unwrap_or("").trim();
                let a = artist.unwrap_or("").trim();
                if !t.is_empty() || !a.is_empty() {
                    let mut s = String::new();
                    if !a.is_empty() && !t.is_empty() {
                        s.push_str(&format!("{}: {}. ", a, t));
                    } else if !t.is_empty() {
                        s.push_str(&format!("{}. ", t));
                    } else if !a.is_empty() {
                        s.push_str(&format!("{}. ", a));
                    }
                    s.push_str("Текст песни, lyrics, song.");
                    Some(s)
                } else {
                    None
                }
            }
        };

        // 3. Run the installed model first and emit JSON confidence data too.
        let temp_json = temp_out_base.with_extension("json");
        let run_whisper = |model: &Path, output_base: &Path| {
            let mut cmd = tokio::process::Command::new(&whisper_exe);
            cmd.kill_on_drop(true)
                .arg("-m").arg(model)
                .arg("-f").arg(&temp_wav)
                .args([
                    "-olrc", "-ojf",
                    "-l", "auto",
                    "-t", "4",
                    "-np",
                    "-sns",
                    "-sow",
                    "-ml", "48",
                    "--no-speech-thold", "0.65"
                ])
                .arg("-of").arg(output_base);
            if let Some(ref p) = prompt_arg {
                cmd.arg("--prompt").arg(p);
            }
            if precise {
                cmd.args(["-nfa", "-dtw", "large.v3.turbo"]);
            }
            #[cfg(windows)]
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            cmd
        };

        let whisper_timeout = if precise { 900 } else { 120 };
        let first_status = tokio::time::timeout(std::time::Duration::from_secs(whisper_timeout), run_whisper(&model_path, &temp_out_base).status()).await
            .map_err(|_| format!("Таймаут распознавания Whisper ({} сек)", whisper_timeout))?
            .map_err(|e| format!("Ошибка запуска Whisper: {}", e))?;

        let first_has_output = first_status.success() && temp_lrc.exists();
        if precise && !first_has_output {
            return Err("Whisper large-v3-turbo не смог завершить распознавание".to_string());
        }

        let first_lrc = if first_has_output {
            fs::read_to_string(&temp_lrc).map_err(|e| format!("Не удалось прочесть результат Whisper: {}", e))?
        } else {
            String::new()
        };
        let first_is_weak = !first_has_output || Self::whisper_output_is_low_quality(&first_lrc, &temp_json, precise);
        let mut raw_lrc = first_lrc;

        // If the fast model produced little or low-confidence text, retry once with small.
        if first_is_weak && !precise && !model_path.ends_with("ggml-small.bin") {
            let small_model = self.ensure_whisper_model_named("small").await?;
            let small_base = temp_dir.path().join("lyrics-small");
            let small_lrc = small_base.with_extension("lrc");
            let small_json = small_base.with_extension("json");
            let small_status = tokio::time::timeout(std::time::Duration::from_secs(300), run_whisper(&small_model, &small_base).status()).await
                .map_err(|_| "Таймаут повторного распознавания Whisper на модели small (300 сек)".to_string())?
                .map_err(|e| format!("Ошибка запуска Whisper small: {}", e))?;
            if !small_status.success() || !small_lrc.exists() {
                return Err("Whisper не смог распознать текст даже на более точной модели small".to_string());
            }
            raw_lrc = fs::read_to_string(&small_lrc).map_err(|e| format!("Не удалось прочесть повторный результат Whisper: {}", e))?;
            if Self::whisper_output_is_low_quality(&raw_lrc, &small_json, false) {
                return Err("Whisper распознал слишком мало разборчивого текста. Результат не сохранён; попробуйте найти готовый текст или другой аудиомикс.".to_string());
            }
        } else if first_is_weak {
            return Err("Whisper распознал слишком мало разборчивого текста. Результат не сохранён.".to_string());
        }

        let clean_lrc = Self::clean_whisper_lrc(&raw_lrc);

        if Self::whisper_output_is_low_quality(&clean_lrc, Path::new(""), false) {
            return Err("В треке не обнаружено разборчивого текста".to_string());
        }

        let lrc = if let Some(ref plain_text) = prompt.filter(|p| p.trim().len() > 30) {
            if let Some(aligned) = Self::align_plain_lyrics_to_timestamps(plain_text, &clean_lrc) {
                aligned
            } else if precise {
                Self::whisper_word_lrc(&temp_json).unwrap_or(clean_lrc)
            } else {
                clean_lrc
            }
        } else if precise {
            Self::whisper_word_lrc(&temp_json).unwrap_or(clean_lrc)
        } else {
            clean_lrc
        };
        if let Ok(mut staged) = tempfile::NamedTempFile::new_in(&cache_dir) {
            if std::io::Write::write_all(&mut staged, lrc.as_bytes()).is_ok() {
                let _ = staged.persist(&cache_file_v3);
            }
        }
        Ok(Self::whisper_lyrics_result(lrc, precise))
    }

    fn whisper_timestamp(ticks: u64) -> String {
        format!("{:02}:{:02}.{:02}", ticks / 6000, ticks / 100 % 60, ticks % 100)
    }

    fn whisper_word_lrc(json_path: &Path) -> Option<String> {
        let data: Value = serde_json::from_slice(&fs::read(json_path).ok()?).ok()?;
        let mut lines = Vec::new();
        for segment in data.get("transcription")?.as_array()? {
            let text = segment.get("text")?.as_str()?.trim();
            if !Self::whisper_has_lyric_words(text) { continue; }
            let offsets = segment.get("offsets")?;
            let from = offsets.get("from")?.as_u64()? / 10;
            let to = offsets.get("to")?.as_u64()? / 10;
            let mut reconstructed = String::new();
            let mut starts = Vec::new();
            let mut word_boundary = true;
            for token in segment.get("tokens").and_then(Value::as_array).into_iter().flatten() {
                let Some(text) = token.get("text").and_then(Value::as_str) else { continue; };
                if text.starts_with("[_") && text.ends_with(']') { continue; }
                let time = token.get("t_dtw").and_then(Value::as_u64).filter(|tick| (from..=to).contains(tick));
                for ch in text.chars() {
                    if word_boundary && !ch.is_whitespace() {
                        starts.push(time);
                    }
                    word_boundary = ch.is_whitespace();
                    reconstructed.push(ch);
                }
            }
            let times = starts.into_iter().collect::<Option<Vec<_>>>();
            let timed = times.filter(|times| !times.is_empty() && times.windows(2).all(|pair| pair[0] <= pair[1])
                && reconstructed.split_whitespace().eq(text.split_whitespace()));
            if let Some(times) = timed {
                let mut line = format!("[{}]", Self::whisper_timestamp(times[0]));
                let has_end = times.last().is_some_and(|last| *last < to);
                for (index, (word, time)) in text.split_whitespace().zip(times).enumerate() {
                    if index > 0 { line.push(' '); }
                    line.push_str(&format!("<{}>{}", Self::whisper_timestamp(time), word));
                }
                if has_end { line.push_str(&format!("<{}>", Self::whisper_timestamp(to))); }
                lines.push(line);
            } else {
                lines.push(format!("[{}]{}", Self::whisper_timestamp(from), text));
            }
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    fn align_plain_lyrics_to_timestamps(plain_lyrics: &str, whisper_lrc: &str) -> Option<String> {
        let official_lines: Vec<&str> = plain_lyrics.lines()
            .map(str::trim)
            .filter(|l| {
                if l.is_empty() { return false; }
                if l.starts_with('[') && l.ends_with(']') { return false; }
                if l.starts_with('(') && l.ends_with(')') && l.to_lowercase().contains("instrumental") { return false; }
                l.chars().any(char::is_alphanumeric)
            })
            .collect();

        if official_lines.is_empty() {
            return None;
        }

        let mut whisper_segments: Vec<(&str, &str, u64)> = Vec::new();
        for line in whisper_lrc.lines() {
            let line = line.trim();
            if let Some((tag, text)) = line.split_once(']') {
                let tag = tag.trim_start_matches('[');
                if let Some((min_str, sec_str)) = tag.split_once(':') {
                    if let (Ok(min), Ok(sec)) = (min_str.parse::<u64>(), sec_str.parse::<f64>()) {
                        let ms = min * 60_000 + (sec * 1000.0) as u64;
                        let text = text.trim();
                        let clean_text = text.split('<').next().unwrap_or(text).trim();
                        if !clean_text.is_empty() {
                            whisper_segments.push((tag, clean_text, ms));
                        }
                    }
                }
            }
        }

        if whisper_segments.is_empty() {
            return None;
        }

        let normalize_words = |s: &str| -> Vec<String> {
            s.to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .map(String::from)
                .collect()
        };

        let word_overlap = |a: &[String], b: &[String]| -> f32 {
            if a.is_empty() || b.is_empty() { return 0.0; }
            let mut matches = 0;
            for w in a {
                if b.contains(w) { matches += 1; }
            }
            (2.0 * matches as f32) / (a.len() + b.len()) as f32
        };

        let official_words: Vec<Vec<String>> = official_lines.iter().map(|l| normalize_words(l)).collect();
        let whisper_words: Vec<Vec<String>> = whisper_segments.iter().map(|(_, t, _)| normalize_words(t)).collect();

        let mut matched_indices: Vec<Option<usize>> = vec![None; official_lines.len()];
        let mut last_whisper_idx = 0;
        let mut match_count = 0;

        for (off_idx, off_w) in official_words.iter().enumerate() {
            if off_w.is_empty() { continue; }
            let mut best_score = 0.28;
            let mut best_idx = None;

            let max_search = (last_whisper_idx + 8).min(whisper_segments.len());
            for w_idx in last_whisper_idx..max_search {
                let score = word_overlap(off_w, &whisper_words[w_idx]);
                if score > best_score {
                    best_score = score;
                    best_idx = Some(w_idx);
                }
            }

            if let Some(idx) = best_idx {
                matched_indices[off_idx] = Some(idx);
                last_whisper_idx = idx;
                match_count += 1;
            }
        }

        if match_count == 0 || match_count * 4 < official_lines.len().min(8) {
            return None;
        }

        let mut final_lrc = Vec::new();
        let mut anchor_time = whisper_segments[matched_indices.iter().flatten().next().copied().unwrap_or(0)].2;

        for (i, &line) in official_lines.iter().enumerate() {
            let timestamp_ms = if let Some(w_idx) = matched_indices[i] {
                anchor_time = whisper_segments[w_idx].2;
                anchor_time
            } else {
                let next_anchor = matched_indices.iter().enumerate()
                    .skip(i + 1)
                    .find_map(|(next_i, opt)| opt.map(|w_idx| (next_i, whisper_segments[w_idx].2)));

                if let Some((next_i, next_time)) = next_anchor {
                    let step = (next_time.saturating_sub(anchor_time)) / ((next_i - i + 1) as u64);
                    anchor_time + step
                } else {
                    anchor_time + 3000
                }
            };

            let mins = timestamp_ms / 60_000;
            let secs = (timestamp_ms % 60_000) / 1000;
            let centis = (timestamp_ms % 1000) / 10;
            final_lrc.push(format!("[{:02}:{:02}.{:02}]{}", mins, secs, centis, line));
        }

        Some(final_lrc.join("\n"))
    }

    fn whisper_lyrics_result(lrc: String, precise: bool) -> Value {
        let plain = lrc.lines()
            .map(|l| if let Some(idx) = l.find(']') { l[idx + 1..].trim() } else { l.trim() })
            .filter(|l| !l.is_empty() && !l.starts_with("[by:"))
            .map(|line| {
                let mut pieces = line.split('<');
                let mut text = pieces.next().unwrap_or_default().to_string();
                for piece in pieces {
                    if let Some((tag, rest)) = piece.split_once('>') {
                        if let Some((minutes, seconds)) = tag.split_once(':') {
                            if minutes.parse::<u64>().is_ok() && seconds.parse::<f64>().is_ok() {
                                text.push_str(rest);
                                continue;
                            }
                        }
                    }
                    text.push('<');
                    text.push_str(piece);
                }
                text
            })
            .collect::<Vec<_>>()
            .join("\n");

        serde_json::json!({
            "syncedLyrics": lrc,
            "plainLyrics": plain,
            "isAi": true,
            "whisperPrecise": precise,
            "whisperRevision": precise.then_some(3)
        })
    }

    pub fn format_artist(u: &Value) -> FormattedArtist {
        let id = u.get("id").cloned().unwrap_or(Value::Null);
        let username = u.get("username").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        let permalink = u.get("permalink").and_then(|v| v.as_str()).unwrap_or("").to_string();
        
        let mut avatar = u.get("avatar_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if avatar.contains("large.jpg") {
            avatar = avatar.replace("large.jpg", "t500x500.jpg");
        }
        if avatar.is_empty() {
            avatar = "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=300".to_string();
        }

        let banner_url = u.get("visuals")
            .and_then(|v| v.get("visuals"))
            .and_then(|arr| arr.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("visual_url"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        let followers_count = u.get("followers_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let track_count = u.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let description = u.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        
        let verified = u.get("verified").and_then(|v| v.as_bool()).unwrap_or_else(|| {
            u.get("badges").and_then(|b| b.get("verified")).and_then(|v| v.as_bool()).unwrap_or(false)
        });

        let city = u.get("city").and_then(|v| v.as_str()).map(|s| s.to_string());
        let country = u.get("country_code").and_then(|v| v.as_str()).map(|s| s.to_string());
        let permalink_url = u.get("permalink_url").and_then(|v| v.as_str()).map(|s| s.to_string());

        FormattedArtist {
            id,
            username,
            permalink,
            avatar_url: avatar,
            banner_url,
            followers_count,
            track_count,
            description,
            verified,
            city,
            country,
            permalink_url,
        }
    }

    pub fn format_album(a: &Value) -> FormattedAlbum {
        let id = a.get("id").cloned().unwrap_or(Value::Null);
        let title = a.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Album").to_string();
        let artist = a.get("user").and_then(|u| u.get("username")).and_then(|v| v.as_str()).unwrap_or("Unknown Artist").to_string();
        let mut cover = a.get("artwork_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if cover.contains("large.jpg") {
            cover = cover.replace("large.jpg", "t500x500.jpg");
        }
        let track_count = a.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let release_year = a.get("release_date")
            .or_else(|| a.get("created_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.get(0..4))
            .map(|s| s.to_string());
        let permalink_url = a.get("permalink_url").and_then(|v| v.as_str()).map(|s| s.to_string());

        FormattedAlbum {
            id,
            title,
            artist,
            cover,
            track_count,
            release_year,
            permalink_url,
        }
    }

    pub async fn search_artists(&self, query: &str, limit: Option<usize>) -> Result<Vec<FormattedArtist>, String> {
        let clean = query.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }
        let client_id = self.get_client_id().await;
        let lim = limit.unwrap_or(5);
        let url = format!(
            "https://api-v2.soundcloud.com/search/users?q={}&limit={}&client_id={}",
            urlencoding_simple(clean),
            lim,
            client_id
        );
        let cfg = self.load_config();
        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header(&cfg) {
            req = req.header("Authorization", auth);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            let mut artists = Vec::new();
            if let Some(arr) = data.get("collection").and_then(|v| v.as_array()) {
                for u in arr {
                    artists.push(Self::format_artist(u));
                }
            }
            Ok(artists)
        } else {
            Err(format!("Search artists failed: status {}", res.status()))
        }
    }

    pub async fn get_artist(&self, query: &str) -> Result<FormattedArtist, String> {
        let clean = query.trim();
        if clean.is_empty() {
            return Err("Artist query cannot be empty".to_string());
        }
        let client_id = self.get_client_id().await;
        let cfg = self.load_config();

        let is_numeric = clean.chars().all(|c| c.is_ascii_digit());
        let url = if is_numeric {
            format!("https://api-v2.soundcloud.com/users/{}?client_id={}", clean, client_id)
        } else {
            format!(
                "https://api-v2.soundcloud.com/search/users?q={}&limit=5&client_id={}",
                urlencoding_simple(clean),
                client_id
            )
        };

        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header(&cfg) {
            req = req.header("Authorization", auth);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            if is_numeric {
                Ok(Self::format_artist(&data))
            } else {
                let collection = data.get("collection").and_then(|v| v.as_array());
                if let Some(arr) = collection {
                    if !arr.is_empty() {
                        let clean_lower = clean.to_lowercase();
                        // 1. Exact verified match (e.g. "The Smiths", "Alex G", "MF Doom")
                        let best = arr.iter().find(|u| {
                            let un = u.get("username").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                            let pl = u.get("permalink").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                            let ver = u.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
                            ver && (un == clean_lower || pl == clean_lower)
                        })
                        // 2. Exact username / permalink match
                        .or_else(|| arr.iter().find(|u| {
                            let un = u.get("username").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                            let pl = u.get("permalink").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                            un == clean_lower || pl == clean_lower
                        }))
                        // 3. Any verified artist
                        .or_else(|| arr.iter().find(|u| {
                            u.get("verified").and_then(|v| v.as_bool()).unwrap_or(false)
                        }))
                        // 4. Fall back to user with the highest follower count
                        .or_else(|| arr.iter().max_by_key(|u| {
                            u.get("followers_count").and_then(|v| v.as_u64()).unwrap_or(0)
                        }))
                        .unwrap_or(&arr[0]);

                        return Ok(Self::format_artist(best));
                    }
                }
                Err(format!("Artist '{}' not found", clean))
            }
        } else {
            Err(format!("Get artist failed: status {}", res.status()))
        }
    }

    pub async fn get_artist_tracks(&self, user_id: Value, limit: Option<usize>) -> Result<Vec<FormattedTrack>, String> {
        let client_id = self.get_client_id().await;
        let cfg = self.load_config();
        let id_str = if let Some(n) = user_id.as_i64() {
            n.to_string()
        } else if let Some(s) = user_id.as_str() {
            s.trim_matches('"').to_string()
        } else {
            user_id.to_string().trim_matches('"').to_string()
        };
        let lim = limit.unwrap_or(50);

        // 1. Try /toptracks first so "Популярные треки" actually shows the artist's real top hits
        let toptracks_url = format!(
            "https://api-v2.soundcloud.com/users/{}/toptracks?limit={}&client_id={}",
            id_str,
            lim,
            client_id
        );
        let mut req = self.http.get(&toptracks_url);
        if let Some(auth) = self.auth_header(&cfg) {
            req = req.header("Authorization", auth);
        }
        if let Ok(res) = req.send().await {
            if res.status().is_success() {
                if let Ok(data) = res.json::<Value>().await {
                    let mut tracks = Vec::new();
                    if let Some(arr) = data.get("collection").and_then(|v| v.as_array()) {
                        for t in arr {
                            tracks.push(Self::format_track(t, false));
                        }
                    }
                    if !tracks.is_empty() {
                        return Ok(tracks);
                    }
                }
            }
        }

        // 2. Fall back to /tracks if /toptracks is empty or unsupported
        let url = format!(
            "https://api-v2.soundcloud.com/users/{}/tracks?limit={}&client_id={}",
            id_str,
            lim,
            client_id
        );
        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header(&cfg) {
            req = req.header("Authorization", auth);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        if res.status().is_success() {
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            let mut tracks = Vec::new();
            if let Some(arr) = data.get("collection").and_then(|v| v.as_array()) {
                for t in arr {
                    tracks.push(Self::format_track(t, false));
                }
            }
            Ok(tracks)
        } else {
            Err(format!("Get artist tracks failed: status {}", res.status()))
        }
    }

    pub async fn get_artist_albums(&self, user_id: Value, limit: Option<usize>) -> Result<Vec<FormattedAlbum>, String> {
        let client_id = self.get_client_id().await;
        let cfg = self.load_config();
        let id_str = if let Some(n) = user_id.as_i64() {
            n.to_string()
        } else if let Some(s) = user_id.as_str() {
            s.trim_matches('"').to_string()
        } else {
            user_id.to_string().trim_matches('"').to_string()
        };
        let lim = limit.unwrap_or(20);
        let url = format!(
            "https://api-v2.soundcloud.com/users/{}/albums?limit={}&client_id={}",
            id_str,
            lim,
            client_id
        );
        let mut req = self.http.get(&url);
        if let Some(auth) = self.auth_header(&cfg) {
            req = req.header("Authorization", auth);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let mut albums = Vec::new();
        if res.status().is_success() {
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            if let Some(arr) = data.get("collection").and_then(|v| v.as_array()) {
                for a in arr {
                    albums.push(Self::format_album(a));
                }
            }
        }
        if albums.is_empty() {
            let pl_url = format!(
                "https://api-v2.soundcloud.com/users/{}/playlists_without_albums?limit={}&client_id={}",
                id_str,
                lim,
                client_id
            );
            let mut pl_req = self.http.get(&pl_url);
            if let Some(auth) = self.auth_header(&cfg) {
                pl_req = pl_req.header("Authorization", auth);
            }
            if let Ok(pl_res) = pl_req.send().await {
                if pl_res.status().is_success() {
                    if let Ok(pl_data) = pl_res.json::<Value>().await {
                        if let Some(arr) = pl_data.get("collection").and_then(|v| v.as_array()) {
                            for a in arr {
                                albums.push(Self::format_album(a));
                            }
                        }
                    }
                }
            }
        }
        Ok(albums)
    }








    pub async fn get_liked_tracks(&self, limit: usize) -> Result<Vec<FormattedTrack>, String> {
        let _refresh_guard = self.likes_refresh.lock().await;
        let (cfg, revision_at_start) = {
            let revision = self.likes_revision.lock().unwrap();
            (self.load_config(), *revision)
        };
        let client_id = self.get_client_id().await;
        let mut token_opt = self.auth_header(&cfg);

        // 1. Determine or auto-resolve numeric user ID
        let mut active_user_id = if let Some(ref uid) = cfg.user_id {
            if let Some(n) = uid.as_i64() {
                n.to_string()
            } else if let Some(s) = uid.as_str() {
                s.to_string()
            } else {
                uid.to_string().trim_matches(&['"', '\''][..]).to_string()
            }
        } else if let Some(ref prof) = cfg.user_profile {
            prof.get("id")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
                .unwrap_or_default()
        } else if let Some(ref raw_tok) = cfg.oauth_token {
            Self::extract_user_id_from_token(raw_tok).unwrap_or_default()
        } else {
            String::new()
        };

        if (active_user_id.is_empty() || active_user_id == "me" || active_user_id == "1000000") && token_opt.is_some() {
            if let Ok((uid_num, _)) = self.ensure_active_user().await {
                active_user_id = uid_num.to_string();
            }
        }

        // 2. Build candidate start endpoints in order of priority:
        let mut candidate_endpoints = Vec::new();
        if !active_user_id.is_empty() && active_user_id != "me" {
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/users/{}/track_likes?client_id={}&limit=50&linked_partitioning=1",
                active_user_id, client_id
            ));
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/users/{}/likes?client_id={}&limit=50&linked_partitioning=1",
                active_user_id, client_id
            ));
        }
        if token_opt.is_some() {
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/me/track_likes?client_id={}&limit=50&linked_partitioning=1",
                client_id
            ));
        }

        let mut all_tracks: Vec<FormattedTrack> = Vec::new();
        let target_limit = if limit == 0 { 500 } else { limit };
        let mut fetched = false;
        let mut complete = false;

        'endpoints: for start_url in candidate_endpoints {
            all_tracks.clear();
            let mut next_url = Some(start_url);
            let mut pages = 0;

            while let Some(url) = next_url {
                pages += 1;
                if pages > 20 || all_tracks.len() >= target_limit {
                    break;
                }

                let mut req = self.http.get(&url)
                    .header("User-Agent", USER_AGENT)
                    .header("Accept", "application/json")
                    .header("Origin", "https://soundcloud.com")
                    .header("Referer", "https://soundcloud.com/");

                if let Some(ref tok) = token_opt {
                    req = req.header("Authorization", tok);
                }

                let mut res = match req.send().await {
                    Ok(r) => r,
                    Err(_) => continue 'endpoints,
                };

                if res.status() == reqwest::StatusCode::UNAUTHORIZED && token_opt.is_some() {
                    // Token rejected with 401: drop token and retry public endpoint without Authorization
                    token_opt = None;
                    if !url.contains("/me/") && !url.contains("/me?") {
                        let retry_req = self.http.get(&url)
                            .header("User-Agent", USER_AGENT)
                            .header("Accept", "application/json")
                            .header("Origin", "https://soundcloud.com")
                            .header("Referer", "https://soundcloud.com/");
                        if let Ok(r) = retry_req.send().await {
                            res = r;
                        }
                    }
                }

                if !res.status().is_success() {
                    continue 'endpoints;
                }

                let body: Value = match res.json().await {
                    Ok(j) => j,
                    Err(_) => continue 'endpoints,
                };

                let collection = body.get("collection").and_then(|v| v.as_array());
                if let Some(items) = collection {
                    for item in items {
                        let track_obj = if item.get("track").is_some() {
                            item.get("track").unwrap()
                        } else {
                            item
                        };

                        if track_obj.get("id").is_some() && track_obj.get("title").is_some() {
                            let tid = track_obj.get("id").cloned().unwrap_or(Value::Null);
                            if !all_tracks.iter().any(|t| t.id == tid) {
                                all_tracks.push(Self::format_track(track_obj, true));
                            }
                        }
                    }
                } else {
                    continue 'endpoints;
                }

                let next = body.get("next_href").and_then(Value::as_str);
                if next.is_some_and(|url| !is_soundcloud_api_url(url)) { break; }
                next_url = next.map(|s| {
                    if !s.contains("client_id=") {
                        let sep = if s.contains('?') { "&" } else { "?" };
                        format!("{}{}client_id={}", s, sep, client_id)
                    } else {
                        s.to_string()
                    }
                });
                if next_url.is_none() {
                    complete = true;
                    break;
                }
            }

            fetched = true;
            break;
        }

        let revision = self.likes_revision.lock().unwrap();
        let current_cfg = self.load_config();
        let account = Self::likes_account(&current_cfg);
        let mut cache = self.load_likes_cache(&current_cfg);
        let save_account = if !account.is_empty() { account.clone() } else { active_user_id.clone() };
        if fetched && (active_user_id == account || account.is_empty()) && current_cfg.oauth_token == cfg.oauth_token {
            cache.update_remote(all_tracks, &save_account, complete, *revision == revision_at_start);
            self.save_likes_cache(&cache)?;
        }
        Ok(cache.visible_tracks(&save_account))
    }

    pub fn find_yt_dlp() -> std::path::PathBuf {



        if let Ok(exe) = std::env::current_exe() {



            if let Some(dir) = exe.parent() {



                let cand = dir.join("yt-dlp.exe");



                if cand.exists() {



                    return cand;



                }



            }



        }



        let cand_f = std::path::PathBuf::from(r"f:\Soundcloud\yt-dlp.exe");



        if cand_f.exists() {



            return cand_f;



        }



        let cand_wg = std::path::PathBuf::from(r"C:\Users\games\AppData\Local\Microsoft\WinGet\Packages\yt-dlp.yt-dlp_Microsoft.Winget.Source_8wekyb3d8bbwe\yt-dlp.exe");



        if cand_wg.exists() {



            return cand_wg;



        }



        std::path::PathBuf::from("yt-dlp")



    }







    pub async fn resolve_youtube_stream_by_id(video_id: &str) -> Option<String> {



        let clean_id = video_id.trim();



        if clean_id.len() != 11 || !clean_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {



            return None;



        }







        let yt_url = format!("https://www.youtube.com/watch?v={}", clean_id);



        let yt_bin = Self::find_yt_dlp();







        let mut stream_cmd = tokio::process::Command::new(&yt_bin);



        stream_cmd.args([



            "--no-warnings",



            "--no-playlist",



            "-g",



            "-f",



            "bestaudio[ext=m4a]/140/bestaudio",



            &yt_url,



        ]);



        #[cfg(windows)]



        stream_cmd.creation_flags(0x08000000);







        let res_stream = tokio::time::timeout(std::time::Duration::from_secs(14), stream_cmd.output()).await;



        if let Ok(Ok(out)) = res_stream {



            if out.status.success() {



                let stdout = String::from_utf8_lossy(&out.stdout);



                if let Some(url) = stdout.lines().find(|l| l.starts_with("http")) {



                    return Some(url.trim().to_string());



                }



            }



        }



        None



    }







    pub async fn resolve_official_youtube_stream(artist: &str, title: &str, duration_ms: u64) -> Option<String> {
        if artist.is_empty() && title.is_empty() {
            return None;
        }

        let clean_title = title.trim();
        let clean_artist = artist.trim();
        let query = if !clean_artist.is_empty() && !clean_title.is_empty() {
            format!("ytsearch8:{} {}", clean_artist, clean_title)
        } else {
            format!("ytsearch8:{}", clean_title)
        };

        let cache_key = format!(
            "{}|{}|{}",
            clean_artist.to_lowercase(),
            clean_title.to_lowercase(),
            duration_ms
        );
        let yt_cache = YT_SEARCH_CACHE
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
        if let Some((cached_id, cached_at)) = yt_cache
            .lock()
            .ok()
            .and_then(|c| c.get(&cache_key).cloned())
        {
            if cached_at.elapsed() < std::time::Duration::from_secs(6 * 3600) {
                if let Some(raw_url) = Self::resolve_youtube_stream_by_id(&cached_id).await {
                    return crate::auth_server::register_youtube_stream(&raw_url);
                }
            }
        }

        let yt_bin = Self::find_yt_dlp();
        let mut search_cmd = tokio::process::Command::new(&yt_bin);
        search_cmd.args([
            "--no-warnings",
            "--no-playlist",
            "--flat-playlist",
            "--print",
            "%(id)s\t%(duration)s\t%(title)s\t%(uploader)s",
            &query,
        ]);

        #[cfg(windows)]
        search_cmd.creation_flags(0x08000000);

        let search_res = tokio::time::timeout(std::time::Duration::from_secs(10), search_cmd.output()).await;
        if let Ok(Ok(out)) = search_res {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let target_sec = duration_ms as f64 / 1000.0;
                let lower_orig_title = clean_title.to_lowercase();
                let lower_orig_artist = clean_artist.to_lowercase();
                let unwanted_keywords = ["live", "cover", "remix", "reaction", "tutorial", "karaoke", "instrumental", "hour"];

                let mut best_id: Option<String> = None;
                let mut best_score: i32 = -1;

                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.is_empty() {
                        continue;
                    }
                    let vid_id = parts[0].trim();
                    if vid_id.is_empty() || vid_id == "NA" {
                        continue;
                    }
                    let dur_sec: f64 = parts.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                    let v_title = parts.get(2).map(|s| s.trim()).unwrap_or("");
                    let uploader = parts.get(3).map(|s| s.trim()).unwrap_or("");
                    let lower_v_title = v_title.to_lowercase();
                    let lower_uploader = uploader.to_lowercase();

                    let diff = if target_sec > 0.0 && dur_sec > 0.0 {
                        (dur_sec - target_sec).abs()
                    } else {
                        0.0
                    };

                    // Hard duration tolerance (within 8 sec)
                    if target_sec > 0.0 && diff > 8.0 {
                        continue;
                    }

                    // Check for unwanted keywords if original doesn't have them
                    let mut has_unwanted = false;
                    for kw in &unwanted_keywords {
                        if lower_v_title.contains(kw) && !lower_orig_title.contains(kw) {
                            has_unwanted = true;
                            break;
                        }
                    }
                    if has_unwanted {
                        continue;
                    }

                    let mut score: i32 = 0;

                    // 1. Official artist channel / Topic priority (+100)
                    if !lower_orig_artist.is_empty() && (lower_uploader == lower_orig_artist || lower_uploader.contains(&lower_orig_artist)) {
                        score += 100;
                    } else if lower_uploader.ends_with("- topic") || lower_uploader.contains("topic") {
                        score += 60;
                    }

                    // 2. Exact Title match (+50)
                    let clean_vt = lower_v_title
                        .replace(&format!("{} - ", lower_orig_artist), "")
                        .replace(&format!("{} – ", lower_orig_artist), "")
                        .trim()
                        .to_string();
                    if clean_vt == lower_orig_title {
                        score += 50;
                    }

                    // 3. Duration closeness (+30)
                    if diff <= 2.0 {
                        score += 30;
                    } else if diff <= 5.0 {
                        score += 15;
                    }

                    if score > best_score {
                        best_score = score;
                        best_id = Some(vid_id.to_string());
                    }
                }

                if let Some(id_ref) = best_id.as_ref() {
                    if let Ok(mut c) = yt_cache.lock() {
                        c.insert(cache_key.clone(), (id_ref.clone(), std::time::Instant::now()));
                    }
                }
                if let Some(id) = best_id {
                    if let Some(raw_url) = Self::resolve_youtube_stream_by_id(&id).await {
                        return crate::auth_server::register_youtube_stream(&raw_url);
                    }
                }
            }
        }

        None
    }







    async fn public_track_request(
        &self,
        url: &str,
        auth: &mut Option<&str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let request = || {
            self.http.get(url)
                .header("Origin", "https://soundcloud.com")
                .header("Referer", "https://soundcloud.com/")
        };
        if let Some(header) = *auth {
            let response = request().header("Authorization", header).send().await?;
            if response.status() != reqwest::StatusCode::UNAUTHORIZED {
                return Ok(response);
            }
            // Public track requests can continue without an expired session.
            *auth = None;
        }
        request().send().await
    }

    pub async fn resolve_stream_url(&self, track_id: Value) -> Result<String, String> {



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;







        let (id_str, opt_title, opt_artist, _opt_duration, opt_provider) = if let Some(obj) = track_id.as_object() {



            let id = obj.get("id").map(|v| {



                if let Some(n) = v.as_i64() {



                    n.to_string()



                } else if let Some(s) = v.as_str() {



                    s.to_string()



                } else {



                    v.to_string().trim_matches('"').to_string()



                }



            }).unwrap_or_default();



            let title = obj.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());



            let artist = obj.get("artist").and_then(|v| v.as_str()).map(|s| s.to_string());



            let dur = obj.get("duration").and_then(|v| v.as_u64());



            let prov = obj.get("provider").and_then(|v| v.as_str()).map(|s| s.to_string());



            (id, title, artist, dur, prov)



        } else {



            let id = if let Some(n) = track_id.as_i64() {



                n.to_string()



            } else if let Some(s) = track_id.as_str() {



                s.to_string()



            } else {



                track_id.to_string().trim_matches('"').to_string()



            };



            (id, None, None, None, None)



        };







        // 1. YouTube direct routing (provider == "youtube" or 11-char video ID)



        let is_yt_id = id_str.len() == 11 && !id_str.chars().all(|c| c.is_ascii_digit()) && !id_str.contains(' ');



        if opt_provider.as_deref() == Some("youtube") || is_yt_id {



            if is_yt_id {



                if let Some(audio_url) = Self::resolve_youtube_stream_by_id(&id_str).await {



                    let proxied = crate::auth_server::register_youtube_stream(&audio_url)
                        .ok_or_else(|| "Недопустимый адрес аудио YouTube".to_string())?;



                    return Ok(proxied);



                }



            }



            let title = opt_title.as_deref().unwrap_or("");



            let artist = opt_artist.as_deref().unwrap_or("");



            if !title.is_empty() {



                if let Some(audio_url) = Self::resolve_official_youtube_stream(artist, title, 0).await {



                    // resolve_official_youtube_stream already returns a 127.0.0.1 proxy URL:



                    return Ok(audio_url);



                }



            }



            return Err("Не удалось получить аудиопоток YouTube Music".to_string());



        }







        // 2. Yandex Music / Spotify routing



        if opt_provider.as_deref() == Some("yandex") || opt_provider.as_deref() == Some("spotify") {



            let title = opt_title.as_deref().unwrap_or("");



            let artist = opt_artist.as_deref().unwrap_or("");



            if !title.is_empty() {



                if let Some(audio_url) = Self::resolve_official_youtube_stream(artist, title, 0).await {



                    // resolve_official_youtube_stream already returns a 127.0.0.1 proxy URL:



                    return Ok(audio_url);



                }



            }



            return Err("Не удалось получить аудиопоток для трека".to_string());



        }







        // 3. SOUNDCLOUD ROUTING ONLY — STRICTLY NO SUBSTITUTION, NO COVERS



        let mut client_ids = std::collections::VecDeque::from([client_id]);
        if client_ids.front().is_some_and(|id| id != DEFAULT_CLIENT_ID) {
            client_ids.push_back(DEFAULT_CLIENT_ID.to_string());
        }
        let mut refreshed_client_id = false;
        let session_auth = self.auth_header(&cfg);
        let mut auth = session_auth.as_deref();
        let mut last_err = "No track found".to_string();

        if !id_str.is_empty() {
            while let Some(cid) = client_ids.pop_front() {
                let url = format!("https://api-v2.soundcloud.com/tracks/{}?client_id={}", id_str, cid);
                let res = match self.public_track_request(&url, &mut auth).await {
                    Ok(res) => res,
                    Err(e) => {
                        last_err = e.to_string();
                        continue;
                    }
                };

                if !res.status().is_success() {
                    let st = res.status().as_u16();
                    last_err = format!("HTTP {}", st);
                    if st == 429 {
                        return Err(format!("SoundCloud: не удалось получить аудиопоток ({last_err})"));
                    }
                    if (st == 401 || st == 403) && !refreshed_client_id {
                        refreshed_client_id = true;
                        if let Some(id) = self.refresh_dynamic_client_id().await {
                            if id != cid && !client_ids.contains(&id) {
                                client_ids.push_front(id);
                            }
                        }
                    }
                    continue;
                }

                let track: Value = match res.json().await {



                    Ok(j) => j,



                    Err(e) => {



                        last_err = e.to_string();



                        continue;



                    }



                };







                let policy = track.get("policy").and_then(|v| v.as_str()).unwrap_or("");



                let mon = track.get("monetization_model").and_then(|v| v.as_str()).unwrap_or("");



                let is_restricted = policy == "SNIP" || policy == "BLOCK" || mon.starts_with("SUB_");







                if !is_restricted {



                    if let Some(transcodings) = track.get("media")



                        .and_then(|m| m.get("transcodings"))



                        .and_then(|t| t.as_array()) {







                        let track_auth = track.get("track_authorization").and_then(|v| v.as_str());







                        let mut unencrypted_candidates = Vec::new();



                        for tc in transcodings {



                            let proto = tc.get("format").and_then(|f| f.get("protocol")).and_then(|p| p.as_str()).unwrap_or("");



                            if proto == "progressive" && !proto.contains("encrypted") {



                                unencrypted_candidates.push(tc);



                            }



                        }



                        for tc in transcodings {



                            let proto = tc.get("format").and_then(|f| f.get("protocol")).and_then(|p| p.as_str()).unwrap_or("");



                            if proto == "hls" && !proto.contains("encrypted") {



                                unencrypted_candidates.push(tc);



                            }



                        }



                        for tc in transcodings {



                            let proto = tc.get("format").and_then(|f| f.get("protocol")).and_then(|p| p.as_str()).unwrap_or("");



                            if !proto.contains("encrypted") && proto != "progressive" && proto != "hls" {



                                unencrypted_candidates.push(tc);



                            }



                        }







                        for tc in unencrypted_candidates {



                            if let Some(stream_req_url) = tc.get("url").and_then(|v| v.as_str()).filter(|url| is_soundcloud_api_url(url)) {



                                let sep = if stream_req_url.contains('?') { "&" } else { "?" };



                                let full_stream_req = match track_auth {



                                    Some(auth) if !auth.is_empty() => format!("{}{}client_id={}&track_authorization={}", stream_req_url, sep, cid, auth),



                                    _ => format!("{}{}client_id={}", stream_req_url, sep, cid),



                                };



                                if let Ok(s_res) = self.public_track_request(&full_stream_req, &mut auth).await {
                                    if s_res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                                        return Err("SoundCloud: слишком много запросов к аудиопотоку (HTTP 429)".to_string());
                                    }
                                    if s_res.status().is_success() {



                                        if let Ok(s_json) = s_res.json::<Value>().await {



                                            if let Some(audio_url) = s_json.get("url").and_then(|v| v.as_str()) {



                                                return Ok(audio_url.to_string());



                                            }



                                        }



                                    }



                                }



                            }



                        }



                    }



                }







                // If track is DRM protected or transcodings failed, fallback to verified official YouTube audio
                let track_title = track.get("title").and_then(|v| v.as_str()).or(opt_title.as_deref()).unwrap_or("");
                let track_artist = track.get("user").and_then(|u| u.get("username")).and_then(|v| v.as_str()).or(opt_artist.as_deref()).unwrap_or("");
                let track_dur = track.get("duration").and_then(|v| v.as_u64()).or(_opt_duration).unwrap_or(0);

                if !track_title.is_empty() {
                    if let Some(yt_stream) = Self::resolve_official_youtube_stream(track_artist, track_title, track_dur).await {
                        return Ok(yt_stream);
                    }
                }

                return Err("SoundCloud: трек защищён DRM и недоступен для воспроизведения".to_string());



            }



        }







        if let Some(ref t) = opt_title {
            let a = opt_artist.as_deref().unwrap_or("");
            if let Some(yt_stream) = Self::resolve_official_youtube_stream(a, t, _opt_duration.unwrap_or(0)).await {
                return Ok(yt_stream);
            }
        }

        Err(format!("SoundCloud: не удалось получить аудиопоток ({})", last_err))



    }







pub async fn login_account(&self, identifier: String, token: Option<String>) -> Result<Value, String> {



        let raw = identifier.trim();



        let clean = raw.strip_prefix('@').unwrap_or(raw).trim();



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;







        let mut user_data: Option<Value> = None;



        let mut resolved_token: Option<String> = token.clone();







        let is_url = clean.starts_with("http://")



            || clean.starts_with("https://")



            || clean.contains("soundcloud.com");







        let is_identifier_token = !is_url && (



            clean.starts_with("eyJ")



            || clean.starts_with("2-")



            || clean.starts_with("OAuth ")



            || clean.starts_with("Bearer ")



            || (clean.contains('-') && clean.len() > 50 && !clean.contains('/') && !clean.contains('.'))



            || clean.len() > 70



        );







        if is_identifier_token {



            resolved_token = Some(clean.to_string());



        }







        // 1. If we have a token (either explicit or from identifier), ALWAYS try /me first!
        if let Some(ref tok) = resolved_token {
            let tr = tok.trim().trim_matches(&['"', '\''][..]).trim();
            if !tr.is_empty() {
                let auth_h = if tr.starts_with("OAuth ") || tr.starts_with("Bearer ") {
                    let parts: Vec<&str> = tr.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let scheme = parts[0];
                        let raw = parts[1].trim().trim_matches(&['"', '\''][..]).trim();
                        format!("{} {}", scheme, raw)
                    } else {
                        tr.to_string()
                    }
                } else {
                    format!("OAuth {}", tr)
                };



                let url = format!("https://api-v2.soundcloud.com/me?client_id={}", client_id);



                if let Ok(res) = self.http.get(&url)



                    .header("Authorization", &auth_h)



                    .header("User-Agent", USER_AGENT)



                    .header("Origin", "https://soundcloud.com")



                    .header("Referer", "https://soundcloud.com/")



                    .send()



                    .await 



                {



                    if res.status().is_success() {



                        user_data = res.json().await.ok();



                    }



                }



            }



        }







        // 2. If no user from token and identifier is NOT a token:



        if user_data.is_none() && !is_identifier_token && !clean.is_empty() {



            // A. Numeric user ID



            if let Ok(uid_num) = clean.parse::<i64>() {



                let url = format!("https://api-v2.soundcloud.com/users/{}?client_id={}", uid_num, client_id);



                if let Ok(res) = self.http.get(&url)



                    .header("User-Agent", USER_AGENT)



                    .header("Origin", "https://soundcloud.com")



                    .header("Referer", "https://soundcloud.com/")



                    .send()



                    .await 



                {



                    if res.status().is_success() {



                        user_data = res.json().await.ok();



                    }



                }



            }







            // B. Extract username / slug if it's a URL



            let search_slug = if is_url {



                clean.trim_end_matches('/').rsplit('/').next().unwrap_or(clean)



            } else {



                clean



            };







            // C. Try resolve URL



            if user_data.is_none() {



                let direct_url = if clean.starts_with("http://") || clean.starts_with("https://") {



                    clean.to_string()



                } else if clean.contains("soundcloud.com/") {



                    format!("https://{}", clean)



                } else {



                    format!("https://soundcloud.com/{}", search_slug)



                };



                if let Ok(resolved) = self.resolve_url(&direct_url).await {



                    let kind = resolved.get("kind").and_then(|v| v.as_str()).unwrap_or("");



                    if kind == "user" {



                        user_data = Some(resolved);



                    }



                }



            }







            // D. Search users by search_slug



            if user_data.is_none() && !search_slug.is_empty() {



                let search_url = format!("https://api-v2.soundcloud.com/search/users?q={}&client_id={}&limit=10", urlencoding_simple(search_slug), client_id);



                if let Ok(res) = self.http.get(&search_url)



                    .header("User-Agent", USER_AGENT)



                    .header("Origin", "https://soundcloud.com")



                    .header("Referer", "https://soundcloud.com/")



                    .send()



                    .await 



                {



                    if res.status().is_success() {



                        if let Ok(search_res) = res.json::<Value>().await {



                            if let Some(items) = search_res.get("collection").and_then(|v| v.as_array()) {



                                let matched = items.iter().find(|u| {



                                    u.get("username").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case(search_slug)).unwrap_or(false)



                                        || u.get("permalink").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case(search_slug)).unwrap_or(false)



                                }).or_else(|| items.first());







                                if let Some(u) = matched {



                                    user_data = Some(u.clone());



                                }



                            }



                        }



                    }



                }



            }



        }







        let u_val = match user_data {
            Some(u) => u,
            None => {
                // A supplied token that SoundCloud rejected must never fall back to a
                // previously cached profile: doing so silently resurrects an old account.
                if resolved_token.is_some() {
                    return Err(
                        "Токен SoundCloud недействителен или истёк. Войдите в аккаунт заново."
                            .to_string(),
                    );
                }

                if !clean.is_empty() {
                    let clean_name = clean.to_string();
                    let fake_id = clean.parse::<i64>().unwrap_or(1000000);
                    serde_json::json!({
                        "id": fake_id,
                        "username": clean_name,
                        "permalink": clean_name,
                        "avatar_url": "https://a1.sndcdn.com/images/default_avatar_large.png",
                        "track_count": 0,
                        "public_favorites_count": 0,
                        "followers_count": 0,
                        "followings_count": 0
                    })
                } else if let Some(ref prof) = cfg.user_profile {
                    prof.clone()
                } else {
                    return Err("Пользователь не найден. Проверьте никнейм, ID, ссылку или токен.".to_string());
                }
            }



        };







        let mut updated_cfg = self.load_config();



        updated_cfg.user_id = u_val.get("id").cloned();



        updated_cfg.user_profile = Some(u_val.clone());



        if let Some(tok) = resolved_token {
            let tr = tok.trim().trim_matches(&['"', '\''][..]).trim();
            if !tr.is_empty() {
                let clean_tok = if tr.starts_with("OAuth ") {
                    tr[6..].trim().trim_matches(&['"', '\''][..]).trim().to_string()
                } else if tr.starts_with("Bearer ") {
                    tr[7..].trim().trim_matches(&['"', '\''][..]).trim().to_string()
                } else {
                    tr.to_string()
                };
                updated_cfg.oauth_token = Some(clean_tok);
            }
        }



        self.save_raw_config(&updated_cfg)?;







        // Fetch liked tracks immediately (up to 500 tracks)



        let likes = self.get_liked_tracks(500).await.unwrap_or_else(|_| self.load_local_likes());



        if !likes.is_empty() {



            updated_cfg.cached_likes = Some(likes.clone());



            let _ = self.save_config(&updated_cfg);



        }







        Ok(serde_json::json!({



            "profile": u_val,



            "likes": likes,



            "config": updated_cfg



        }))



    }







    pub async fn set_oauth_token(&self, token: &str) -> Result<Value, String> {
        let clean = token.trim().trim_matches(&['"', '\''][..]).trim();
        if clean.is_empty() {
            return Err("Token is empty".to_string());
        }

        // Validate against SoundCloud first: a rejected token must never be persisted,
        // otherwise the UI keeps reporting a stale "signed in" state.
        match self.login_account(String::new(), Some(clean.to_string())).await {
            Ok(data) => {
                let mut cfg = self.load_config();
                cfg.oauth_token = Some(clean.to_string());
                let _ = self.save_raw_config(&cfg);
                Ok(data)
            }
            Err(e) => {
                let mut cfg = self.load_config();
                if cfg.oauth_token.as_deref() == Some(clean) {
                    cfg.oauth_token = None;
                    let _ = self.save_raw_config(&cfg);
                }
                Err(e)
            }
        }
    }

    pub async fn toggle_like(&self, mut track: FormattedTrack, should_like: bool) -> Result<Option<String>, String> {
        let id = LikesCache::track_id(&track.id);
        if track.id.is_null() || id.is_empty() {
            return Err("У трека нет идентификатора".to_string());
        }

        // Preserve the order of consecutive likes and unlikes on SoundCloud.
        let _request_guard = self.like_requests.lock().await;
        let cfg = {
            let mut revision = self.likes_revision.lock().unwrap();
            let cfg = self.load_config();
            let mut cache = self.load_likes_cache(&cfg);
            let account = Self::likes_account(&cfg);
            cache.account.get_or_insert_with(|| account.clone());
            let changes = cache.changes.entry(account).or_default();
            changes.retain(|change| LikesCache::track_id(&change.id) != id);
            track.liked = should_like;
            changes.insert(0, track);
            self.save_likes_cache(&cache)?;
            *revision += 1;
            cfg
        };

        if !id.bytes().all(|byte| byte.is_ascii_digit()) {
            return Ok(None);
        }
        let account = Self::likes_account(&cfg);
        let Some(saved_token) = self.auth_header(&cfg) else {
            return Ok((!account.is_empty()).then(|| "Для синхронизации войдите в SoundCloud".to_string()));
        };
        if self.is_like_rate_limited() {
            return Ok(Some("SoundCloud временно ограничил лайки (429)".to_string()));
        }

        let (user_id, token) = match self.ensure_active_user().await {
            Ok(session) => session,
            Err(err) => return Ok(Some(err)),
        };
        if saved_token != token || (!account.is_empty() && account != user_id.to_string()) {
            return Ok(Some("Сессия SoundCloud изменилась".to_string()));
        }

        let client_id = self.get_client_id().await;
        let url = format!(
            "https://api-v2.soundcloud.com/users/{user_id}/track_likes/{id}?client_id={client_id}"
        );
        let request = if should_like {
            self.http.put(&url).body("")
        } else {
            self.http.delete(&url)
        };
        let response = request
            .header("Authorization", &token)
            .header("Origin", "https://soundcloud.com")
            .header("Referer", "https://soundcloud.com/")
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .send()
            .await;

        let Ok(response) = response else {
            return Ok(Some("SoundCloud недоступен".to_string()));
        };
        let status = response.status().as_u16();
        let reason = match status {
            200..=299 => return Ok(None),
            404 if !should_like => return Ok(None),
            401 => "Войдите в SoundCloud заново".to_string(),
            403 => {
                if response.text().await.unwrap_or_default().to_ascii_lowercase().contains("captcha") {
                    "SoundCloud запрашивает CAPTCHA. Подтвердите действие на сайте SoundCloud".to_string()
                } else {
                    "SoundCloud отклонил изменение (403)".to_string()
                }
            }
            429 => {
                self.set_like_rate_limited(std::time::Duration::from_secs(1800));
                "SoundCloud временно ограничил лайки (429)".to_string()
            }
            _ => format!("SoundCloud отклонил изменение ({status})"),
        };
        Ok(Some(reason))
    }

    pub async fn resolve_url(&self, target_url: &str) -> Result<Value, String> {



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let encoded = urlencoding_simple(target_url);



        let url = format!("https://api-v2.soundcloud.com/resolve?url={}&client_id={}", encoded, client_id);







        let mut req = self.http.get(&url)



            .header("User-Agent", USER_AGENT)



            .header("Referer", "https://soundcloud.com/");



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await.map_err(|e| e.to_string())?;



        if res.status().is_success() {



            let data: Value = res.json().await.map_err(|e| e.to_string())?;



            Ok(data)



        } else {



            Err(format!("Resolve failed: HTTP {}", res.status()))



        }



    }







    pub async fn search_tracks(&self, query: &str) -> Result<Vec<FormattedTrack>, String> {



        let clean = query.trim();



        // Support direct SoundCloud links (tracks, playlists, sets, artists)



        if clean.starts_with("http://") || clean.starts_with("https://") || clean.starts_with("soundcloud.com") {



            let full_url = if clean.starts_with("soundcloud.com") {



                format!("https://{}", clean)



            } else {



                clean.to_string()



            };



            if let Ok(resolved) = self.resolve_url(&full_url).await {



                let kind = resolved.get("kind").and_then(|v| v.as_str()).unwrap_or("");



                if kind == "track" {



                    return Ok(vec![Self::format_track(&resolved, false)]);



                } else if kind == "playlist" {



                    if let Some(items) = resolved.get("tracks").and_then(|v| v.as_array()) {



                        let tracks: Vec<FormattedTrack> = items.iter()



                            .filter(|t| t.get("id").is_some() && t.get("title").is_some())



                            .map(|t| Self::format_track(t, false))



                            .collect();



                        if !tracks.is_empty() {



                            return Ok(tracks);



                        }



                    }



                }



            }



        }







        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let encoded = urlencoding_simple(query);



        let url = format!("https://api-v2.soundcloud.com/search/tracks?q={}&client_id={}&limit=30", encoded, client_id);







        let session_auth = self.auth_header(&cfg);
        let mut auth = session_auth.as_deref();
        let res = self.public_track_request(&url, &mut auth).await.map_err(|e| e.to_string())?;

        if res.status().is_success() {



            let data: Value = res.json().await.map_err(|e| e.to_string())?;



            let items = data.get("collection").and_then(|v| v.as_array()).cloned().unwrap_or_default();



            let tracks: Vec<FormattedTrack> = items



                .into_iter()



                .filter(|t| t.get("id").is_some() && t.get("title").is_some())



                .map(|t| Self::format_track(&t, false))



                .collect();



            Ok(tracks)



        } else {



            Err(format!("Search failed: HTTP {}", res.status()))



        }



    }







    pub async fn resolve_tracks(&self, ids: Vec<Value>) -> Result<Vec<FormattedTrack>, String> {



        if ids.is_empty() {



            return Ok(Vec::new());



        }



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let id_strs: Vec<String> = ids.iter().take(40).filter_map(|id| {



            if let Some(n) = id.as_i64() {



                Some(n.to_string())



            } else if let Some(s) = id.as_str() {



                let clean = s.trim_matches('"');



                if clean.is_empty() || clean == "null" { None } else { Some(clean.to_string()) }



            } else {



                let s = id.to_string().trim_matches('"').to_string();



                if s.is_empty() || s == "null" { None } else { Some(s) }



            }



        }).collect();







        if id_strs.is_empty() {



            return Ok(Vec::new());



        }



        let joined = id_strs.join(",");



        let url = format!("https://api-v2.soundcloud.com/tracks?ids={}&client_id={}", joined, client_id);







        let auth = self.auth_header(&cfg);
        let mut req = self.http.get(&url);
        if let Some(token) = &auth {
            req = req.header("Authorization", token);
        }

        let mut res = req.send().await.map_err(|e| e.to_string())?;
        if res.status() == reqwest::StatusCode::UNAUTHORIZED && auth.is_some() {
            res = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        }

        if res.status().is_success() {



            let items: Vec<Value> = res.json().await.map_err(|e| e.to_string())?;



            let tracks = items



                .into_iter()



                .filter(|t| t.get("id").is_some() && t.get("title").is_some())



                .map(|t| Self::format_track(&t, false))



                .collect();



            Ok(tracks)



        } else {



            Err(format!("Resolve tracks failed: HTTP {}", res.status()))



        }



    }







    pub async fn get_mixed_selections(&self) -> Result<Value, String> {



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let ts = std::time::SystemTime::now()



            .duration_since(std::time::UNIX_EPOCH)



            .map(|d| d.as_secs())



            .unwrap_or(0);



        let url = format!("https://api-v2.soundcloud.com/mixed-selections?client_id={}&_t={}", client_id, ts);







        let mut req = self.http.get(&url);



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await;



        if let Ok(r) = res {



            if r.status().is_success() {



                if let Ok(data) = r.json::<Value>().await {



                    let mut updated_cfg = cfg.clone();



                    updated_cfg.cached_mixed = Some(data.clone());



                    updated_cfg.mixed_updated_at = Some(ts);



                    let _ = self.save_config(&updated_cfg);



                    return Ok(data);



                }



            }



        }







        if let Some(cached) = cfg.cached_mixed {



            Ok(cached)



        } else {



            Err("Failed to fetch mixed selections and no cache available".to_string())



        }



    }







    pub async fn get_system_playlist(&self, urn: &str) -> Result<Value, String> {



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let url = format!(



            "https://api-v2.soundcloud.com/system-playlists/{}?client_id={}",



            urlencoding_simple(urn),



            client_id



        );







        let mut req = self.http.get(&url);



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await.map_err(|e| e.to_string())?;



        if res.status().is_success() {



            let playlist_data: Value = res.json().await.map_err(|e| e.to_string())?;



            let tracks_val = playlist_data.get("tracks").and_then(|v| v.as_array()).cloned().unwrap_or_default();







            let mut formatted_tracks = Vec::new();



            let mut ids_to_resolve = Vec::new();







            for t in &tracks_val {



                if t.get("title").is_some() && t.get("media").is_some() {



                    formatted_tracks.push(Self::format_track(t, false));



                } else if let Some(id) = t.get("id") {



                    ids_to_resolve.push(id.clone());



                }



            }







            if !ids_to_resolve.is_empty() {



                if let Ok(resolved) = self.resolve_tracks(ids_to_resolve).await {



                    formatted_tracks.extend(resolved);



                }



            }







            if formatted_tracks.len() > 21 {



                formatted_tracks.truncate(21);



            }



            Ok(serde_json::json!({



                "playlist": playlist_data,



                "tracks": formatted_tracks



            }))



        } else {
            Err(format!("System playlist fetch failed: HTTP {}", res.status()))
        }
    }

    pub async fn get_playlist_tracks(&self, playlist_id: u64) -> Result<Vec<FormattedTrack>, String> {
        let cfg = self.load_config();
        let client_id = self.get_client_id().await;
        let url = format!(
            "https://api-v2.soundcloud.com/playlists/{}?client_id={}&representation=full",
            playlist_id, client_id
        );
        let request = || self.http.get(&url).header("User-Agent", USER_AGENT);
        let auth = self.auth_header(&cfg);
        let mut req = request();
        if let Some(token) = &auth {
            req = req.header("Authorization", token);
        }
        let mut res = req.send().await.map_err(|_| "SoundCloud недоступен".to_string())?;
        if res.status() == reqwest::StatusCode::UNAUTHORIZED && auth.is_some() {
            res = request().send().await.map_err(|_| "SoundCloud недоступен".to_string())?;
        }
        if !res.status().is_success() {
            return Err(format!("SoundCloud вернул HTTP {}", res.status()));
        }

        let playlist: Value = res.json().await.map_err(|_| "Некорректный ответ SoundCloud".to_string())?;
        let tracks = playlist.get("tracks").and_then(Value::as_array)
            .ok_or_else(|| "SoundCloud не вернул треки плейлиста".to_string())?;
        let mut missing_ids = Vec::new();
        let mut by_id = std::collections::HashMap::new();
        for track in tracks {
            if let Some(id) = track.get("id") {
                if track.get("title").and_then(Value::as_str).is_some() {
                    by_id.insert(id.to_string(), Self::format_track(track, false));
                } else {
                    missing_ids.push(id.clone());
                }
            }
        }
        for batch in missing_ids.chunks(50) {
            for track in self.resolve_tracks(batch.to_vec()).await? {
                by_id.insert(track.id.to_string(), track);
            }
        }

        Ok(tracks.iter()
            .filter_map(|track| by_id.get(&track.get("id")?.to_string()).cloned())
            .collect())
    }

    pub async fn get_user_playlists(&self) -> Result<Vec<Value>, String> {
        let cfg = self.load_config();
        let client_id = self.get_client_id().await;
        let token_opt = self.auth_header(&cfg);

        let mut active_user_id = if let Some(ref uid) = cfg.user_id {
            if let Some(n) = uid.as_i64() {
                n.to_string()
            } else if let Some(s) = uid.as_str() {
                s.to_string()
            } else {
                uid.to_string().trim_matches(&['"', '\''][..]).to_string()
            }
        } else if let Some(ref prof) = cfg.user_profile {
            prof.get("id")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
                .unwrap_or_default()
        } else if let Some(ref raw_tok) = cfg.oauth_token {
            Self::extract_user_id_from_token(raw_tok).unwrap_or_default()
        } else {
            String::new()
        };

        if active_user_id.parse::<i64>().is_err() && token_opt.is_some() {
            active_user_id = self.ensure_active_user().await?.0.to_string();
        }
        if active_user_id.parse::<i64>().is_err() {
            return Err("Подключите аккаунт SoundCloud".to_string());
        }

        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        let url = format!(
            "https://api-v2.soundcloud.com/users/{}/playlist_likes?client_id={}&limit=50&linked_partitioning=1",
            active_user_id, client_id
        );
        let request = || self.http.get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .header("Origin", "https://soundcloud.com")
            .header("Referer", "https://soundcloud.com/");

        let mut req = request();
        if let Some(ref token) = token_opt {
            req = req.header("Authorization", token);
        }
        let mut res = req.send().await.map_err(|_| "SoundCloud недоступен".to_string())?;
        if res.status() == reqwest::StatusCode::UNAUTHORIZED && token_opt.is_some() {
            res = request().send().await.map_err(|_| "SoundCloud недоступен".to_string())?;
        }
        if !res.status().is_success() {
            return Err(format!("SoundCloud вернул HTTP {}", res.status()));
        }
        let body: Value = res.json().await.map_err(|_| "Некорректный ответ SoundCloud".to_string())?;

        let collection = body.get("collection").and_then(|v| v.as_array())
            .or_else(|| body.as_array());

        if let Some(items) = collection {
            for item in items {
                let p = if item.get("playlist").is_some() {
                    item.get("playlist").unwrap()
                } else {
                    item
                };

                let pid = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                if pid > 0 && !seen_ids.contains(&pid) {
                    seen_ids.insert(pid);

                    let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("SoundCloud Playlist");
                    let artwork = p.get("artwork_url").and_then(|v| v.as_str())
                        .or_else(|| p.get("user").and_then(|u| u.get("avatar_url")).and_then(|v| v.as_str()))
                        .unwrap_or("");
                    let cover = artwork.replace("-large.", "-t500x500.");
                    let author = p.get("user").and_then(|u| u.get("username").or_else(|| u.get("permalink"))).and_then(|v| v.as_str()).unwrap_or("SoundCloud");
                    let track_count = p.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let dur_ms = p.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
                    let hours = dur_ms / 3600000;
                    let mins = (dur_ms % 3600000) / 60000;
                    let duration_str = if hours > 0 {
                        format!("{} ч {} мин", hours, mins)
                    } else {
                        format!("{} мин", mins)
                    };

                    let tracks_arr = p.get("tracks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    let mut formatted_tracks = Vec::new();
                    for t in tracks_arr {
                        if t.get("title").is_some() {
                            formatted_tracks.push(Self::format_track(&t, false));
                        }
                    }

                    results.push(serde_json::json!({
                        "id": pid.to_string(),
                        "title": title,
                        "author": author,
                        "desc": p.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                        "cover": cover,
                        "track_count": if track_count > 0 { track_count } else { formatted_tracks.len() as u64 },
                        "duration_str": duration_str,
                        "tracks": formatted_tracks
                    }));
                }
            }
        }

        Ok(results)
    }







    pub async fn get_related_tracks(&self, track_id: Value) -> Result<Vec<FormattedTrack>, String> {



        let cfg = self.load_config();



        let client_id = self.get_client_id().await;



        let id_str = if let Some(n) = track_id.as_i64() {



            n.to_string()



        } else if let Some(s) = track_id.as_str() {



            s.trim_matches('"').to_string()



        } else {



            track_id.to_string().trim_matches('"').to_string()



        };



        let url = format!("https://api-v2.soundcloud.com/tracks/{}/related?client_id={}&limit=30", id_str, client_id);







        let session_auth = self.auth_header(&cfg);
        let mut auth = session_auth.as_deref();
        let res = self.public_track_request(&url, &mut auth).await.map_err(|e| e.to_string())?;

        if res.status().is_success() {



            let data: Value = res.json().await.map_err(|e| e.to_string())?;



            let items = data.get("collection").and_then(|v| v.as_array()).cloned().unwrap_or_default();



            let tracks: Vec<FormattedTrack> = items



                .into_iter()



                .filter(|t| t.get("id").is_some() && t.get("title").is_some())



                .map(|t| Self::format_track(&t, false))



                .collect();



            Ok(tracks)



        } else {



            Err(format!("Related tracks fetch failed: HTTP {}", res.status()))



        }



    }







    async fn lrclib_get(http: &reqwest::Client, title: &str, artist: &str, dur: Option<f64>) -> Option<Value> {
        if title.is_empty() { return None; }
        let ua = "LrcLibX/1.0.0 (contact@example.com)";
        let mut q: Vec<(&str, &str)> = vec![("track_name", title), ("artist_name", artist)];
        let dur_str = dur.map(|d| (d as u64).to_string());
        if let Some(ref d) = dur_str { q.push(("duration", d.as_str())); }
        if let Ok(Ok(res)) = tokio::time::timeout(
            std::time::Duration::from_millis(1800),
            http.get("https://lrclib.net/api/get").query(&q).header("User-Agent", ua).send()
        ).await {
            if res.status().is_success() {
                return res.json::<Value>().await.ok();
            }
        }
        None
    }

    async fn lrclib_search(http: &reqwest::Client, query: &str) -> Option<Vec<Value>> {
        if query.trim().is_empty() { return None; }
        let ua = "LrcLibX/1.0.0 (contact@example.com)";
        if let Ok(Ok(res)) = tokio::time::timeout(
            std::time::Duration::from_millis(1800),
            http.get("https://lrclib.net/api/search").query(&[("q", query)]).header("User-Agent", ua).send()
        ).await {
            if res.status().is_success() {
                return res.json::<Vec<Value>>().await.ok();
            }
        }
        None
    }

    pub async fn fetch_lyrics(&self, title: String, artist: String, duration: Option<f64>) -> Result<Value, String> {
        let clean_title = clean_lyrics_text(&title);
        let clean_artist = clean_lyrics_text(&artist);

        let mut alt_artist = clean_artist.clone();
        let mut alt_title = clean_title.clone();
        if title.contains(" - ") {
            let parts: Vec<&str> = title.splitn(2, " - ").collect();
            if parts.len() == 2 {
                alt_artist = clean_lyrics_text(parts[0]);
                alt_title = clean_lyrics_text(parts[1]);
            }
        } else if title.contains(" — ") {
            let parts: Vec<&str> = title.splitn(2, " — ").collect();
            if parts.len() == 2 {
                alt_artist = clean_lyrics_text(parts[0]);
                alt_title = clean_lyrics_text(parts[1]);
            }
        } else if title.contains(" – ") {
            let parts: Vec<&str> = title.splitn(2, " – ").collect();
            if parts.len() == 2 {
                alt_artist = clean_lyrics_text(parts[0]);
                alt_title = clean_lyrics_text(parts[1]);
            }
        }

        let http = self.http.clone();
        let c_title = clean_title.clone();
        let c_artist = clean_artist.clone();
        let a_title = alt_title.clone();
        let a_artist = alt_artist.clone();
        let dur_val = duration;

        // 1. Parallel Task A: LRCLIB (concurrent exact + search)
        let lrclib_task = {
            let http = http.clone();
            let c_title = c_title.clone();
            let c_artist = c_artist.clone();
            let a_title = a_title.clone();
            let a_artist = a_artist.clone();
            async move {
                let search_q1 = format!("{} {}", c_title, c_artist).trim().to_string();
                let search_q2 = if !a_title.is_empty() && a_title != c_title {
                    format!("{} {}", a_title, a_artist).trim().to_string()
                } else {
                    c_title.clone()
                };

                let t1 = Self::lrclib_get(&http, &c_title, &c_artist, dur_val);
                let t2 = async {
                    if !a_title.is_empty() && (a_title != c_title || a_artist != c_artist) {
                        Self::lrclib_get(&http, &a_title, &a_artist, dur_val).await
                    } else {
                        None
                    }
                };
                let t3 = Self::lrclib_search(&http, &search_q1);
                let t4 = async {
                    if !search_q2.is_empty() && search_q2 != search_q1 {
                        Self::lrclib_search(&http, &search_q2).await
                    } else {
                        None
                    }
                };

                let (r1, r2, r3, r4) = tokio::join!(t1, t2, t3, t4);

                let is_synced = |v: &Value| -> bool {
                    v.get("syncedLyrics").and_then(|s| s.as_str()).map(|s| s.trim().len() > 15).unwrap_or(false)
                };
                let is_plain = |v: &Value| -> bool {
                    v.get("plainLyrics").and_then(|s| s.as_str()).map(|s| s.trim().len() > 15).unwrap_or(false)
                };
                let dur_diff = |v: &Value| -> f64 {
                    if let Some(target_dur) = dur_val {
                        if let Some(cand_dur) = v.get("duration").and_then(|d| d.as_f64()) {
                            return (cand_dur - target_dur).abs();
                        }
                    }
                    0.0
                };

                if let Some(ref data) = r1 {
                    if (is_synced(data) || is_plain(data)) && dur_diff(data) <= 12.0 {
                        return Some(data.clone());
                    }
                }
                if let Some(ref data) = r2 {
                    if (is_synced(data) || is_plain(data)) && dur_diff(data) <= 12.0 {
                        return Some(data.clone());
                    }
                }

                let mut candidates: Vec<Value> = Vec::new();
                if let Some(items) = r3 { candidates.extend(items); }
                if let Some(items) = r4 { candidates.extend(items); }

                if let Some(hit) = candidates.iter().find(|c| is_synced(c) && dur_diff(c) <= 8.0) {
                    return Some(hit.clone());
                }
                if let Some(hit) = candidates.iter().find(|c| is_synced(c) && dur_diff(c) <= 16.0) {
                    return Some(hit.clone());
                }
                if let Some(hit) = candidates.iter().find(|c| is_plain(c) && dur_diff(c) <= 16.0) {
                    return Some(hit.clone());
                }
                if let Some(hit) = candidates.iter().find(|c| is_synced(c) && dur_diff(c) <= 25.0) {
                    return Some(hit.clone());
                }
                if let Some(hit) = candidates.iter().find(|c| is_plain(c) && dur_diff(c) <= 25.0) {
                    return Some(hit.clone());
                }
                None
            }
        };

        // 2. Parallel Task B: Genius Search
        let http_g = self.http.clone();
        let g_title = if !clean_title.is_empty() { clean_title.clone() } else { title.clone() };
        let g_artist = if !clean_artist.is_empty() { clean_artist.clone() } else { artist.clone() };
        let genius_task = async move {
            Self::fetch_genius_lyrics(&http_g, &g_title, &g_artist).await
        };

        // 3. Parallel Task C: QQ Music (Tencent global synced lyrics database)
        let http_q = self.http.clone();
        let q_title = clean_title.clone();
        let q_artist = clean_artist.clone();
        let qq_task = async move {
            Self::fetch_qq_lyrics(&http_q, &q_title, &q_artist, dur_val).await
        };

        // Run LRCLIB, QQ Music, and Genius concurrently!
        let (lrclib_res, qq_res, genius_res) = tokio::join!(lrclib_task, qq_task, genius_task);

        // 1. If LRCLIB found synced lyrics, prioritize it!
        if let Some(ref lrc_data) = lrclib_res {
            if lrc_data.get("syncedLyrics").and_then(|s| s.as_str()).map(|s| s.trim().len() > 15).unwrap_or(false) {
                return Ok(lrc_data.clone());
            }
        }

        // 2. If QQ Music found synced lyrics, prioritize it!
        if let Some(ref qq_data) = qq_res {
            if qq_data.get("syncedLyrics").and_then(|s| s.as_str()).map(|s| s.trim().len() > 15).unwrap_or(false) {
                return Ok(qq_data.clone());
            }
        }

        // 3. If Genius found lyrics, return it immediately
        if let Some(genius_lyrics) = genius_res {
            let mut obj = serde_json::Map::new();
            obj.insert("plainLyrics".to_string(), Value::String(genius_lyrics));
            obj.insert("source".to_string(), Value::String("genius".to_string()));
            obj.insert("referenceMatched".to_string(), Value::Bool(true));
            obj.insert("trackName".to_string(), Value::String(clean_title.clone()));
            obj.insert("artistName".to_string(), Value::String(clean_artist.clone()));
            return Ok(Value::Object(obj));
        }

        // 4. If LRCLIB found plain lyrics, return it
        if let Some(lrc_data) = lrclib_res {
            return Ok(lrc_data);
        }

        // 5. If QQ Music found plain lyrics, return it
        if let Some(qq_data) = qq_res {
            return Ok(qq_data);
        }

        Err("Lyrics not found".to_string())
    }

    pub async fn fetch_qq_lyrics(http: &reqwest::Client, title: &str, artist: &str, duration: Option<f64>) -> Option<Value> {
        let query = format!("{} {}", artist, title).trim().to_string();
        if query.is_empty() { return None; }

        let search_url = "https://c.y.qq.com/soso/fcgi-bin/client_search_cp";
        let res = tokio::time::timeout(
            std::time::Duration::from_millis(2500),
            http.get(search_url)
                .query(&[("w", query.as_str()), ("format", "json"), ("p", "1"), ("n", "5")])
                .header("Referer", "https://y.qq.com")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .send()
        ).await.ok()?.ok()?;

        if !res.status().is_success() { return None; }
        let data: Value = res.json().await.ok()?;
        let songs = data.get("data")?.get("song")?.get("list")?.as_array()?;

        for song in songs {
            let songmid = song.get("songmid")?.as_str()?;
            let song_dur = song.get("interval").and_then(Value::as_f64);
            if let (Some(target_dur), Some(cand_dur)) = (duration, song_dur) {
                if (target_dur - cand_dur).abs() > 6.0 {
                    continue;
                }
            }

            let lyric_url = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
            let lyr_res = tokio::time::timeout(
                std::time::Duration::from_millis(2500),
                http.get(lyric_url)
                    .query(&[("songmid", songmid), ("format", "json"), ("nobase64", "1")])
                    .header("Referer", "https://y.qq.com")
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                    .send()
            ).await.ok()?.ok()?;

            if !lyr_res.status().is_success() { continue; }
            let lyr_data: Value = lyr_res.json().await.ok()?;
            let lyric = lyr_data.get("lyric").and_then(Value::as_str)?;
            if lyric.trim().len() > 20 && lyric.contains('[') {
                let plain = lyric.lines()
                    .map(|l| if let Some(idx) = l.find(']') { l[idx + 1..].trim() } else { l.trim() })
                    .filter(|l| !l.is_empty() && !l.starts_with("[ti:") && !l.starts_with("[ar:") && !l.starts_with("[al:") && !l.starts_with("[by:") && !l.starts_with("[offset:"))
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut obj = serde_json::Map::new();
                obj.insert("syncedLyrics".to_string(), Value::String(lyric.to_string()));
                obj.insert("plainLyrics".to_string(), Value::String(plain));
                obj.insert("source".to_string(), Value::String("qqmusic".to_string()));
                obj.insert("trackName".to_string(), Value::String(title.to_string()));
                obj.insert("artistName".to_string(), Value::String(artist.to_string()));
                obj.insert("duration".to_string(), serde_json::json!(duration.or(song_dur).unwrap_or(0.0)));
                return Some(Value::Object(obj));
            }
        }
        None
    }

    pub async fn fetch_genius_lyrics(http: &reqwest::Client, title: &str, artist: &str) -> Option<String> {
        let query = format!("{} {}", artist, title).trim().to_string();
        if query.is_empty() {
            return None;
        }

        let timeout_dur = std::time::Duration::from_millis(2500);
        let search_res = tokio::time::timeout(
            timeout_dur,
            http.get("https://genius.com/api/search/multi")
                .query(&[("per_page", "5"), ("q", &query)])
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .send()
        ).await.ok()?.ok()?;

        if !search_res.status().is_success() {
            return None;
        }

        let search_json: Value = search_res.json().await.ok()?;
        let sections = search_json.get("response").and_then(|r| r.get("sections")).and_then(|s| s.as_array())?;
        
        let mut song_path: Option<String> = None;
        let title_key = clean_lyrics_text(title).to_lowercase();
        let artist_key = clean_lyrics_text(artist).to_lowercase();
        for sec in sections {
            if sec.get("type").and_then(|t| t.as_str()) == Some("song") {
                if let Some(hits) = sec.get("hits").and_then(|h| h.as_array()) {
                    for hit in hits {
                        if let Some(result) = hit.get("result") {
                            let result_title = result.get("title").and_then(Value::as_str).unwrap_or_default();
                            let result_artist = result.pointer("/primary_artist/name").and_then(Value::as_str).unwrap_or_default();
                            if clean_lyrics_text(result_title).to_lowercase() != title_key
                                || clean_lyrics_text(result_artist).to_lowercase() != artist_key
                            {
                                continue;
                            }
                            if let Some(path) = result.get("path").and_then(|p| p.as_str()) {
                                song_path = Some(path.to_string());
                                break;
                            }
                        }
                    }
                }
            }
            if song_path.is_some() { break; }
        }

        let song_path = song_path?;
        let song_url = format!("https://genius.com{}", song_path);

        let html_res = tokio::time::timeout(
            timeout_dur,
            http.get(&song_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .send()
        ).await.ok()?.ok()?;

        if !html_res.status().is_success() {
            return None;
        }

        let html = html_res.text().await.ok()?;
        let marker = "data-lyrics-container=\"true\"";
        let mut search_from = 0;
        let mut lyrics = String::new();

        while let Some(pos) = html[search_from..].find(marker) {
            let abs_pos = search_from + pos;
            if let Some(tag_end) = html[abs_pos..].find('>') {
                let content_start = abs_pos + tag_end + 1;
                if let Some(div_end) = html[content_start..].find("</div>") {
                    let chunk = &html[content_start..content_start + div_end];
                    let formatted = chunk
                        .replace("<br/>", "\n")
                        .replace("<br>", "\n")
                        .replace("<br />", "\n");

                    let mut in_tag = false;
                    for ch in formatted.chars() {
                        if ch == '<' {
                            in_tag = true;
                        } else if ch == '>' {
                            in_tag = false;
                        } else if !in_tag {
                            lyrics.push(ch);
                        }
                    }
                    lyrics.push_str("\n\n");
                    search_from = content_start + div_end + 6;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let mut cleaned = lyrics
            .replace("&#x27;", "'")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&nbsp;", " ")
            .replace("&apos;", "'")
            .trim()
            .to_string();

        if let Some(c_pos) = cleaned.find("Contributors") {
            if c_pos < 60 {
                if let Some(next_line) = cleaned[c_pos..].find('\n') {
                    cleaned = cleaned[c_pos + next_line..].trim().to_string();
                }
            }
        }

        if cleaned.len() > 20 {
            Some(cleaned)
        } else {
            None
        }
    }
}

fn is_soundcloud_api_url(raw: &str) -> bool {
    tauri::Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https" && url.host_str() == Some("api-v2.soundcloud.com")
            && url.port_or_known_default() == Some(443) && url.username().is_empty() && url.password().is_none()
    })
}

fn is_transcription_stream_url(raw: &str) -> bool {
    let Ok(url) = tauri::Url::parse(raw) else { return false; };
    if !url.username().is_empty() || url.password().is_some() { return false; }
    if crate::auth_server::is_registered_stream_url(&url) { return true; }
    url.scheme() == "https" && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|host| {
            host == "playback.media-streaming.soundcloud.cloud" || host.ends_with(".sndcdn.com") || host.ends_with(".dzcdn.net")
        })
}

fn default_whisper_precise() -> bool {
    true
}

fn clean_lyrics_text(s: &str) -> String {



    let mut res = String::new();



    let mut depth = 0;



    for c in s.chars() {



        if c == '(' || c == '[' || c == '{' {



            depth += 1;



        } else if c == ')' || c == ']' || c == '}' {



            if depth > 0 { depth -= 1; }



        } else if depth == 0 {



            res.push(c);



        }



    }







    let lower = res.to_lowercase();



    let stop_words = [



        " prod.", " prod by", " produced by", " feat.", " feat ", " ft.", " ft ",



        " slowed", " sped up", " speed up", " reverb", " remix", " official audio", " official video",



        " music video", " edit", " / ", " w/ "



    ];



    let mut cut_idx = res.len();



    for sw in stop_words {



        if let Some(pos) = lower.find(sw) {



            if pos < cut_idx {



                cut_idx = pos;



            }



        }



    }



    res.truncate(cut_idx);



    res.trim().to_string()



}







fn urlencoding_simple(s: &str) -> String {



    let mut out = String::new();



    for b in s.bytes() {



        match b {



            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {



                out.push(b as char);



            }



            b' ' => out.push_str("%20"),



            _ => out.push_str(&format!("%{:02X}", b)),



        }



    }



    out



}







#[allow(dead_code)]



fn simple_str_hash(s: &str) -> u64 {



    let mut h: u64 = 5381;



    for b in s.bytes() {



        h = ((h << 5).wrapping_add(h)).wrapping_add(b as u64);



    }



    h



}



