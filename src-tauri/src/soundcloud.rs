use serde::{Deserialize, Serialize};



use serde_json::Value;



use std::fs;



use std::path::{Path, PathBuf};







const DEFAULT_CLIENT_ID: &str = "3GKD969NJWD1eB6gvQJN0GnueYbcWszv";



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
}







pub struct SoundCloudClient {



    http: reqwest::Client,



    config_path: PathBuf,



}







impl SoundCloudClient {



    pub fn http_client(&self) -> &reqwest::Client {



        &self.http



    }







    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {



        let http = reqwest::Client::builder()



            .user_agent(USER_AGENT)



            .build()



            .unwrap_or_default();



        Self {



            http,



            config_path: config_path.as_ref().to_path_buf(),



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
        let l_path = self.likes_path();
        if let Ok(content) = fs::read_to_string(&l_path) {
            if let Ok(tracks) = serde_json::from_str::<Vec<FormattedTrack>>(&content) {
                if !tracks.is_empty() {
                    return tracks;
                }
            }
        }
        let cfg = self.load_config();
        cfg.cached_likes.unwrap_or_default()
    }

    pub fn save_local_likes(&self, likes: &[FormattedTrack]) -> Result<(), String> {
        let l_path = self.likes_path();
        if let Some(parent) = l_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(likes).map_err(|e| e.to_string())?;
        let _ = fs::write(&l_path, json);

        // Also update cached_likes in config
        let mut cfg = self.load_config();
        cfg.cached_likes = Some(likes.to_vec());
        let _ = self.save_config(&cfg);
        Ok(())
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
        let existing = self.load_config();
        let mut to_save = cfg.clone();

        // General settings update: preserve credentials and session
        if to_save.oauth_token.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            if existing.oauth_token.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
                to_save.oauth_token = existing.oauth_token;
            }
        }
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

        let json = serde_json::to_string_pretty(&to_save).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_raw_config(&self, cfg: &AppConfig) -> Result<(), String> {



        let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;



        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;



        Ok(())



    }







    fn auth_header(&self, cfg: &AppConfig) -> Option<String> {



        cfg.oauth_token.as_ref().and_then(|t| {



            let trimmed = t.trim();



            if trimmed.is_empty() {



                None



            } else if trimmed.starts_with("OAuth ") || trimmed.starts_with("Bearer ") {



                Some(trimmed.to_string())



            } else {



                Some(format!("OAuth {}", trimmed))



            }



        })



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



            if cid.len() > 10 && cid != "dbe62de186a100fbdf5bcab837fcfbc2" {



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







        let res = req



            .send()



            .await



            .map_err(|e| e.to_string())?;







        if res.status().is_success() {



            let data: Value = res.json().await.map_err(|e| e.to_string())?;



            let mut updated_cfg = cfg.clone();



            updated_cfg.user_id = data.get("id").cloned();



            updated_cfg.user_profile = Some(data.clone());



            let _ = self.save_config(&updated_cfg);



            Ok(data)



        } else {



            Err(format!("Profile request failed: HTTP {}", res.status()))



        }



    }







    pub fn format_track(t: &Value, is_liked: bool) -> FormattedTrack {



        let id = t.get("id").cloned().unwrap_or(Value::Null);



        let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();



        let artist = t.get("user")



            .and_then(|u| u.get("username"))



            .and_then(|v| v.as_str())



            .unwrap_or("Unknown Artist")



            .to_string();







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



        }



    }







    pub async fn get_liked_tracks(&self, limit: usize) -> Result<Vec<FormattedTrack>, String> {
        let cfg = self.load_config();
        let client_id = self.get_client_id().await;
        let token_opt = self.auth_header(&cfg);
        let existing_cached = cfg.cached_likes.clone().unwrap_or_default();

        // 1. Determine user ID
        let user_id = if let Some(ref uid) = cfg.user_id {
            if let Some(n) = uid.as_i64() {
                n.to_string()
            } else if let Some(s) = uid.as_str() {
                s.to_string()
            } else {
                uid.to_string().trim_matches('"').to_string()
            }
        } else if let Some(ref prof) = cfg.user_profile {
            prof.get("id")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
                .unwrap_or_default()
        } else {
            String::new()
        };

        // 2. Build candidate start endpoints in order of priority:
        let mut candidate_endpoints = Vec::new();
        if token_opt.is_some() {
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/me/likes/tracks?client_id={}&limit=50&linked_partitioning=1",
                client_id
            ));
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/me/likes?client_id={}&limit=50&linked_partitioning=1",
                client_id
            ));
        }
        if !user_id.is_empty() {
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/users/{}/track_likes?client_id={}&limit=50&linked_partitioning=1",
                user_id, client_id
            ));
            candidate_endpoints.push(format!(
                "https://api-v2.soundcloud.com/users/{}/likes?client_id={}&limit=50&linked_partitioning=1",
                user_id, client_id
            ));
        }

        let mut all_tracks: Vec<FormattedTrack> = Vec::new();
        let target_limit = if limit == 0 { 500 } else { limit };

        for start_url in candidate_endpoints {
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

                let res = match req.send().await {
                    Ok(r) => r,
                    Err(_) => break,
                };

                if !res.status().is_success() {
                    break;
                }

                let body: Value = match res.json().await {
                    Ok(j) => j,
                    Err(_) => break,
                };

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

                        if track_obj.get("id").is_some() && track_obj.get("title").is_some() {
                            let tid = track_obj.get("id").cloned().unwrap_or(Value::Null);
                            if !all_tracks.iter().any(|t| t.id == tid) {
                                all_tracks.push(Self::format_track(track_obj, true));
                            }
                        }
                    }

                    // Fast incremental overlap check:
                    // If page 1 intersects with existing cache, merge new tracks and finish in 1 request!
                    if pages == 1 && !existing_cached.is_empty() && !all_tracks.is_empty() {
                        let has_known = all_tracks.iter().any(|t| existing_cached.iter().any(|c| c.id == t.id));
                        if has_known {
                            let mut merged = Vec::new();
                            for t in &all_tracks {
                                if !existing_cached.iter().any(|c| c.id == t.id) {
                                    merged.push(t.clone());
                                }
                            }
                            merged.extend(existing_cached.clone());
                            all_tracks = merged;
                            break;
                        }
                    }
                } else {
                    break;
                }

                next_url = body.get("next_href").and_then(|v| v.as_str()).map(|s| {
                    if !s.contains("client_id=") {
                        let sep = if s.contains('?') { "&" } else { "?" };
                        format!("{}{}client_id={}", s, sep, client_id)
                    } else {
                        s.to_string()
                    }
                });
            }

            if !all_tracks.is_empty() {
                break;
            }
        }

        if !all_tracks.is_empty() {
            let mut updated_cfg = self.load_config();
            updated_cfg.cached_likes = Some(all_tracks.clone());
            let _ = self.save_config(&updated_cfg);
            Ok(all_tracks)
        } else if !existing_cached.is_empty() {
            Ok(existing_cached)
        } else {
            Ok(all_tracks)
        }
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



        if clean_id.is_empty() {



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







    pub async fn resolve_official_youtube_stream(artist: &str, title: &str, _duration_ms: u64) -> Option<String> {



        if artist.is_empty() && title.is_empty() {



            return None;



        }







        let clean_title = title.trim();



        let clean_artist = artist.trim();



        let query = if !clean_artist.is_empty() && !clean_title.is_empty() {



            format!("ytsearch1:{} - {}", clean_artist, clean_title)



        } else {



            format!("ytsearch1:{}", clean_title)



        };







        let yt_bin = Self::find_yt_dlp();



        let mut stream_cmd = tokio::process::Command::new(&yt_bin);



        stream_cmd.args([



            "--no-warnings",



            "--no-playlist",



            "-g",



            "-f",



            "bestaudio[ext=m4a]/140/bestaudio",



            &query,



        ]);



        #[cfg(windows)]



        stream_cmd.creation_flags(0x08000000);







        let res = tokio::time::timeout(std::time::Duration::from_secs(14), stream_cmd.output()).await;



        if let Ok(Ok(out)) = res {



            if out.status.success() {



                let stdout = String::from_utf8_lossy(&out.stdout);



                if let Some(url) = stdout.lines().find(|l| l.starts_with("http")) {



                    return Some(url.trim().to_string());



                }



            }



        }



        None



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



                    let proxied = format!("http://127.0.0.1:49221/stream?url={}", urlencoding_simple(&audio_url));



                    return Ok(proxied);



                }



            }



            let title = opt_title.as_deref().unwrap_or("");



            let artist = opt_artist.as_deref().unwrap_or("");



            if !title.is_empty() {



                if let Some(audio_url) = Self::resolve_official_youtube_stream(artist, title, 0).await {



                    let proxied = format!("http://127.0.0.1:49221/stream?url={}", urlencoding_simple(&audio_url));



                    return Ok(proxied);



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



                    let proxied = format!("http://127.0.0.1:49221/stream?url={}", urlencoding_simple(&audio_url));



                    return Ok(proxied);



                }



            }



            return Err("Не удалось получить аудиопоток для трека".to_string());



        }







        // 3. SOUNDCLOUD ROUTING ONLY — STRICTLY NO SUBSTITUTION, NO COVERS



        let fallback_client_ids = vec![client_id.as_str(), DEFAULT_CLIENT_ID];



        let mut last_err = "No track found".to_string();







        if !id_str.is_empty() {



            for &cid in &fallback_client_ids {



                let url = format!("https://api-v2.soundcloud.com/tracks/{}?client_id={}", id_str, cid);







                let mut req = self.http.get(&url)



                    .header("User-Agent", USER_AGENT)



                    .header("Origin", "https://soundcloud.com")



                    .header("Referer", "https://soundcloud.com/");



                if let Some(auth) = self.auth_header(&cfg) {



                    req = req.header("Authorization", auth);



                }







                let res = match req.send().await {



                    Ok(r) => r,



                    Err(e) => {



                        last_err = e.to_string();



                        continue;



                    }



                };







                if !res.status().is_success() {



                    last_err = format!("HTTP {}", res.status());



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



                            if let Some(stream_req_url) = tc.get("url").and_then(|v| v.as_str()) {



                                let sep = if stream_req_url.contains('?') { "&" } else { "?" };



                                let full_stream_req = match track_auth {



                                    Some(auth) if !auth.is_empty() => format!("{}{}client_id={}&track_authorization={}", stream_req_url, sep, cid, auth),



                                    _ => format!("{}{}client_id={}", stream_req_url, sep, cid),



                                };



                                let mut s_req = self.http.get(&full_stream_req)



                                    .header("User-Agent", USER_AGENT)



                                    .header("Origin", "https://soundcloud.com")



                                    .header("Referer", "https://soundcloud.com/");



                                if let Some(auth) = self.auth_header(&cfg) {



                                    s_req = s_req.header("Authorization", auth);



                                }



                                if let Ok(s_res) = s_req.send().await {



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







                // If track is restricted or transcodings failed, DO NOT SUBSTITUTE!



                return Err("SoundCloud: трек недоступен в вашем регионе (ограничен автором)".to_string());



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



            let tr = tok.trim();



            if !tr.is_empty() {



                let auth_h = if tr.starts_with("OAuth ") || tr.starts_with("Bearer ") {



                    tr.to_string()



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



                if let Some(ref prof) = cfg.user_profile {
                    prof.clone()
                } else if !clean.is_empty() {
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
                } else {
                    return Err("Пользователь не найден. Проверьте никнейм, ID, ссылку или токен.".to_string());
                }



            }



        };







        let mut updated_cfg = self.load_config();



        updated_cfg.user_id = u_val.get("id").cloned();



        updated_cfg.user_profile = Some(u_val.clone());



        if let Some(tok) = resolved_token {



            let tr = tok.trim();



            if !tr.is_empty() {



                updated_cfg.oauth_token = Some(tr.to_string());



            }



        }



        self.save_raw_config(&updated_cfg)?;







        // Fetch liked tracks immediately (up to 500 tracks)



        let likes = match self.get_liked_tracks(500).await {



            Ok(l) if !l.is_empty() => l,



            _ => updated_cfg.cached_likes.clone().unwrap_or_default(),



        };



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







    pub async fn toggle_like(&self, track_id: Value, should_like: bool) -> Result<bool, String> {



        let cfg = self.load_config();



        let token = self.auth_header(&cfg).ok_or("Not logged in")?;



        let client_id = self.get_client_id().await;



        



        let id_str = if let Some(n) = track_id.as_i64() {



            n.to_string()



        } else if let Some(s) = track_id.as_str() {



            s.to_string()



        } else {



            track_id.to_string().trim_matches('"').to_string()



        };







        let user_id = if let Some(ref uid) = cfg.user_id {



            if let Some(n) = uid.as_i64() {



                n.to_string()



            } else if let Some(s) = uid.as_str() {



                s.to_string()



            } else {



                "me".to_string()



            }



        } else {



            "me".to_string()



        };







        let mut success = false;







        // 1. Try /users/{user_id}/track_likes/{id}



        let url1 = format!(



            "https://api-v2.soundcloud.com/users/{}/track_likes/{}?client_id={}",



            user_id, id_str, client_id



        );



        let req1 = if should_like {



            self.http.put(&url1)



        } else {



            self.http.delete(&url1)



        };



        if let Ok(res1) = req1



            .header("Authorization", &token)



            .header("Origin", "https://soundcloud.com")



            .header("Referer", "https://soundcloud.com/")



            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")



            .header("Accept", "application/json, text/javascript, */*; q=0.01")



            .send()



            .await



        {



            let st = res1.status().as_u16();



            if st == 200 || st == 201 || st == 204 {



                success = true;



            }



        }







        // 2. Fallback to /me/track_likes/{id}



        if !success {



            let url2 = format!(



                "https://api-v2.soundcloud.com/me/track_likes/{}?client_id={}",



                id_str, client_id



            );



            let req2 = if should_like {



                self.http.put(&url2)



            } else {



                self.http.delete(&url2)



            };



            if let Ok(res2) = req2



                .header("Authorization", &token)



                .header("Origin", "https://soundcloud.com")



                .header("Referer", "https://soundcloud.com/")



                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")



                .header("Accept", "application/json, text/javascript, */*; q=0.01")



                .send()



                .await



            {



                let st = res2.status().as_u16();



                if st == 200 || st == 201 || st == 204 {



                    success = true;



                }



            }



        }







        // 3. Fallback to /likes/tracks/{id}



        if !success {



            let url3 = format!(



                "https://api-v2.soundcloud.com/likes/tracks/{}?client_id={}",



                id_str, client_id



            );



            let req3 = if should_like {



                self.http.post(&url3)



            } else {



                self.http.delete(&url3)



            };



            if let Ok(res3) = req3



                .header("Authorization", &token)



                .header("Origin", "https://soundcloud.com")



                .header("Referer", "https://soundcloud.com/")



                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")



                .send()



                .await



            {



                let st = res3.status().as_u16();



                if st == 200 || st == 201 || st == 204 {



                    success = true;



                }



            }



        }







        // 4. Update local cached_likes in config.json



        let mut updated_cfg = self.load_config();



        if let Some(ref mut list) = updated_cfg.cached_likes {



            let id_val = if let Ok(num) = id_str.parse::<i64>() {



                serde_json::json!(num)



            } else {



                serde_json::json!(id_str)



            };



            if !should_like {



                list.retain(|t| t.id != id_val);



            }



            let _ = self.save_config(&updated_cfg);



        }







        Ok(success || should_like)



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







        let mut req = self.http.get(&url);



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await.map_err(|e| e.to_string())?;



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







        let mut req = self.http.get(&url);



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await.map_err(|e| e.to_string())?;



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







        let mut req = self.http.get(&url);



        if let Some(auth) = self.auth_header(&cfg) {



            req = req.header("Authorization", auth);



        }







        let res = req.send().await.map_err(|e| e.to_string())?;



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

        // Run LRCLIB and Genius concurrently!
        let (lrclib_res, genius_res) = tokio::join!(lrclib_task, genius_task);

        // 1. If LRCLIB found synced lyrics, prioritize it!
        if let Some(ref lrc_data) = lrclib_res {
            if lrc_data.get("syncedLyrics").and_then(|s| s.as_str()).map(|s| s.trim().len() > 15).unwrap_or(false) {
                return Ok(lrc_data.clone());
            }
        }

        // 2. If Genius found lyrics, return it immediately
        if let Some(genius_lyrics) = genius_res {
            let mut obj = serde_json::Map::new();
            obj.insert("plainLyrics".to_string(), Value::String(genius_lyrics));
            obj.insert("source".to_string(), Value::String("genius".to_string()));
            obj.insert("trackName".to_string(), Value::String(clean_title.clone()));
            obj.insert("artistName".to_string(), Value::String(clean_artist.clone()));
            return Ok(Value::Object(obj));
        }

        // 3. If LRCLIB found plain lyrics, return it
        if let Some(lrc_data) = lrclib_res {
            return Ok(lrc_data);
        }

        Err("Lyrics not found".to_string())
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
        for sec in sections {
            if sec.get("type").and_then(|t| t.as_str()) == Some("song") {
                if let Some(hits) = sec.get("hits").and_then(|h| h.as_array()) {
                    for hit in hits {
                        if let Some(result) = hit.get("result") {
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



