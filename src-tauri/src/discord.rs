use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DISCORD_CLIENT_ID: &str = "802958833214423081"; // Official SoundCloud Discord Client ID

#[derive(Clone)]
pub struct DiscordManager {
    inner: Arc<Mutex<DiscordState>>,
}

#[cfg(target_os = "windows")]
type IpcStream = std::fs::File;

#[cfg(not(target_os = "windows"))]
type IpcStream = std::os::unix::net::UnixStream;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordPlayback {
    title: String,
    artist: String,
    cover: String,
    is_playing: bool,
    is_buffering: bool,
    position: f64,
    duration: f64,
    rate: f64,
}

impl DiscordPlayback {
    fn timestamps(&self, now: f64) -> Option<Value> {
        if !self.is_playing || self.rate <= 0.0 {
            return None;
        }
        let start = (now - self.position.max(0.0) / self.rate).max(0.0);
        let mut timestamps = serde_json::json!({ "start": start.round() as u64 });
        if self.duration > self.position {
            timestamps["end"] = serde_json::json!((now
                + (self.duration - self.position) / self.rate)
                .ceil() as u64);
        }
        Some(timestamps)
    }
}

struct DiscordState {
    pipe: Option<IpcStream>,
    connected: bool,
    enabled: bool,
    client_id: String,
    show_track: bool,
    show_artist: bool,
    show_cover: bool,
    show_status: bool,
    current_activity: Option<Value>,
    last_playback: Option<(DiscordPlayback, f64)>,
}

impl DiscordManager {
    pub fn new() -> Self {
        let mgr = Self {
            inner: Arc::new(Mutex::new(DiscordState {
                pipe: None,
                connected: false,
                enabled: true,
                client_id: DISCORD_CLIENT_ID.to_string(),
                show_track: true,
                show_artist: true,
                show_cover: true,
                show_status: true,
                current_activity: None,
                last_playback: None,
            })),
        };

        let mgr_clone = mgr.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(5));
            let should_reconnect = {
                let state = mgr_clone.inner.lock().unwrap();
                state.enabled && !state.connected
            };

            if should_reconnect {
                let _ = mgr_clone.try_connect();
            }
        });

        mgr
    }

    pub fn set_options(
        &self,
        enabled: bool,
        show_track: bool,
        show_artist: bool,
        show_cover: bool,
        show_status: bool,
        client_id: String,
    ) {
        let mut state = self.inner.lock().unwrap();
        let cid = if client_id.trim().is_empty() {
            DISCORD_CLIENT_ID.to_string()
        } else {
            client_id.trim().to_string()
        };

        let cid_changed = state.client_id != cid;
        if !cid_changed
            && (
                state.enabled,
                state.show_track,
                state.show_artist,
                state.show_cover,
                state.show_status,
            ) == (enabled, show_track, show_artist, show_cover, show_status)
        {
            return;
        }

        state.enabled = enabled;
        state.show_track = show_track;
        state.show_artist = show_artist;
        state.show_cover = show_cover;
        state.show_status = show_status;
        state.client_id = cid;

        if !enabled || cid_changed {
            Self::write_activity(&mut state, &Value::Null);
            state.connected = false;
            state.pipe = None;
        }

        state.current_activity = if enabled {
            state
                .last_playback
                .as_ref()
                .map(|(playback, now)| Self::render_activity(&state, playback, *now))
        } else {
            None
        };
        let activity = state.current_activity.clone();
        drop(state);

        if let Some(activity) = activity {
            self.send_activity_payload(activity);
        }
    }

    pub fn try_connect(&self) -> bool {
        let mut state = self.inner.lock().unwrap();
        if !state.enabled {
            return false;
        }
        if state.connected && state.pipe.is_some() {
            return true;
        }

        let cid = state.client_id.clone();
        #[cfg(target_os = "windows")]
        for i in 0..10 {
            let pipe_path = format!(r"\\.\pipe\discord-ipc-{}", i);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&pipe_path)
            {
                let handshake = serde_json::json!({
                    "v": 1,
                    "client_id": cid
                });
                let payload = handshake.to_string();
                let op = 0u32;
                let len = payload.len() as u32;

                let mut packet = Vec::with_capacity(8 + payload.len());
                packet.extend_from_slice(&op.to_le_bytes());
                packet.extend_from_slice(&len.to_le_bytes());
                packet.extend_from_slice(payload.as_bytes());

                if file.write_all(&packet).is_ok() && file.flush().is_ok() {
                    state.pipe = Some(file);
                    state.connected = true;

                    if let Some(act) = state.current_activity.clone() {
                        drop(state);
                        self.send_activity_payload(act);
                    }
                    return true;
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut prefixes = Vec::new();
            if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                let p = std::path::PathBuf::from(runtime_dir);
                prefixes.push(p.clone());
                prefixes.push(p.join("app/com.discordapp.Discord"));
                prefixes.push(p.join(".flatpak/org.equicord.equibop/xdg-run"));
                prefixes.push(p.join(".flatpak/com.discordapp.Discord/xdg-run"));
            }
            if let Ok(tmp) = std::env::var("TMPDIR") {
                prefixes.push(std::path::PathBuf::from(tmp));
            }
            prefixes.push(std::path::PathBuf::from("/tmp"));

            for prefix in prefixes {
                for i in 0..10 {
                    let socket_path = prefix.join(format!("discord-ipc-{}", i));
                    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
                        let handshake = serde_json::json!({
                            "v": 1,
                            "client_id": cid
                        });
                        let payload = handshake.to_string();
                        let op = 0u32;
                        let len = payload.len() as u32;

                        let mut packet = Vec::with_capacity(8 + payload.len());
                        packet.extend_from_slice(&op.to_le_bytes());
                        packet.extend_from_slice(&len.to_le_bytes());
                        packet.extend_from_slice(payload.as_bytes());

                        if stream.write_all(&packet).is_ok() && stream.flush().is_ok() {
                            state.pipe = Some(stream);
                            state.connected = true;

                            if let Some(act) = state.current_activity.clone() {
                                drop(state);
                                self.send_activity_payload(act);
                            }
                            return true;
                        }
                    }
                }
            }
        }

        state.connected = false;
        state.pipe = None;
        false
    }

    fn render_activity(state: &DiscordState, playback: &DiscordPlayback, now: f64) -> Value {
        let DiscordPlayback {
            title,
            artist,
            cover,
            is_playing,
            position,
            ..
        } = playback;
        let mut activity = serde_json::json!({
            "name": if state.show_status { "ONYX SOUND" } else if !state.show_track || title.trim().is_empty() { "Music" } else { title.trim() },
            "type": 2,
            "status_display_type": 0
        });

        if state.show_track && !title.trim().is_empty() {
            activity["details"] = serde_json::json!(title);
        }
        let mut status = Vec::new();
        if !is_playing && !playback.is_buffering {
            let seconds = position.max(0.0) as u64;
            status.push(format!("На паузе · {}:{:02}", seconds / 60, seconds % 60));
        }
        if state.show_artist && !artist.trim().is_empty() {
            status.push(format!("by {artist}"));
        }
        if state.show_status && playback.is_buffering {
            status.push("Буферизация".to_string());
        }
        if !status.is_empty() {
            activity["state"] = serde_json::json!(status.join(" • "));
        }

        let mut assets = serde_json::Map::new();
        if state.show_cover && !cover.is_empty() {
            assets.insert("large_image".to_string(), serde_json::json!(cover));
            assets.insert(
                "large_text".to_string(),
                serde_json::json!(if state.show_status { "ONYX SOUND" } else if state.show_track { title } else { "Music" }),
            );
        }
        if state.show_status || (!is_playing && !playback.is_buffering) {
            assets.insert(
                "small_image".to_string(),
                serde_json::json!(if !is_playing && !playback.is_buffering { "pause" } else { "play" }),
            );
            assets.insert(
                "small_text".to_string(),
                serde_json::json!(if *is_playing {
                    "Воспроизведение"
                } else if playback.is_buffering {
                    "Буферизация"
                } else {
                    "На паузе"
                }),
            );
        }
        if !assets.is_empty() {
            activity["assets"] = Value::Object(assets);
        }

        if let Some(timestamps) = playback.timestamps(now) {
            activity["timestamps"] = timestamps;
        }
        activity
    }

    pub fn set_activity(&self, playback: DiscordPlayback) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let activity = {
            let mut state = self.inner.lock().unwrap();
            let activity = state
                .enabled
                .then(|| Self::render_activity(&state, &playback, now));
            state.last_playback = Some((playback, now));
            if state.current_activity == activity {
                return;
            }
            state.current_activity = activity.clone();
            activity
        };
        if let Some(activity) = activity {
            self.send_activity_payload(activity);
        }
    }

    fn send_activity_payload(&self, activity: Value) {
        let mut state = self.inner.lock().unwrap();
        if !state.enabled || state.current_activity.as_ref() != Some(&activity) {
            return;
        }
        if !state.connected || state.pipe.is_none() {
            drop(state);
            self.try_connect();
            return;
        }
        Self::write_activity(&mut state, &activity);
    }

    fn write_activity(state: &mut DiscordState, activity: &Value) {
        let Some(pipe) = state.pipe.as_mut() else {
            return;
        };
        let payload = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity
            },
            "nonce": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos().to_string()
        }).to_string();

        let mut packet = Vec::with_capacity(8 + payload.len());
        packet.extend_from_slice(&1u32.to_le_bytes());
        packet.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        packet.extend_from_slice(payload.as_bytes());

        if pipe.write_all(&packet).and_then(|_| pipe.flush()).is_err() {
            state.connected = false;
            state.pipe = None;
        }
    }

    pub fn clear_activity(&self) {
        let mut state = self.inner.lock().unwrap();
        state.last_playback = None;
        if state.current_activity.take().is_some() {
            Self::write_activity(&mut state, &Value::Null);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_pause_in_the_card_with_the_status_badge_disabled() {
        let state = DiscordState {
            pipe: None,
            connected: false,
            enabled: true,
            client_id: DISCORD_CLIENT_ID.to_string(),
            show_track: true,
            show_artist: true,
            show_cover: true,
            show_status: false,
            current_activity: None,
            last_playback: None,
        };
        let mut playback = DiscordPlayback {
            title: "Track".to_string(),
            artist: "Artist".to_string(),
            cover: String::new(),
            is_playing: false,
            is_buffering: false,
            position: 3.75,
            duration: 120.0,
            rate: 1.0,
        };

        let paused = DiscordManager::render_activity(&state, &playback, 1000.0);
        assert_eq!(paused["details"], "Track");
        assert_eq!(paused["state"], "На паузе · 0:03 • by Artist");
        assert_eq!(paused["name"], "Track");
        assert_eq!(paused["assets"]["small_image"], "pause");
        assert!(paused.get("timestamps").is_none());

        playback.is_buffering = true;
        let buffering = DiscordManager::render_activity(&state, &playback, 1000.0);
        assert_eq!(buffering["state"], "by Artist");

        playback.is_buffering = false;
        playback.is_playing = true;
        let resumed = DiscordManager::render_activity(&state, &playback, 1000.0);
        assert_eq!(resumed["state"], "by Artist");
        assert!(resumed.get("assets").is_none());
        assert!(resumed.get("timestamps").is_some());
    }

    #[test]
    fn branding_toggle_preserves_track_details_and_pause_icon() {
        for show_status in [false, true] {
            for is_playing in [false, true] {
                for show_track in [false, true] {
                    let state = DiscordState {
                        pipe: None, connected: false, enabled: true,
                        client_id: DISCORD_CLIENT_ID.to_string(),
                        show_track, show_artist: true, show_cover: true, show_status,
                        current_activity: None, last_playback: None,
                    };
                    let playback = DiscordPlayback {
                        title: "Track".into(), artist: "Artist".into(),
                        cover: "https://example.com/cover.jpg".into(),
                        is_playing, is_buffering: false,
                        position: 10.0, duration: 120.0, rate: 1.0,
                    };
                    let activity = DiscordManager::render_activity(&state, &playback, 1000.0);
                    assert_eq!(activity["name"], if show_status { "ONYX SOUND" } else if show_track { "Track" } else { "Music" });
                    assert_eq!(activity["status_display_type"], 0);
                    assert_eq!(activity.get("details").is_some(), show_track);
                    assert!(activity["state"].as_str().unwrap().contains("by Artist"));
                    assert_eq!(activity.get("timestamps").is_some(), is_playing);
                    if !is_playing {
                        assert_eq!(activity["assets"]["small_image"], "pause");
                        assert_eq!(activity["assets"]["small_text"], "На паузе");
                    } else if show_status {
                        assert_eq!(activity["assets"]["small_image"], "play");
                    } else {
                        assert!(activity["assets"].get("small_image").is_none());
                    }
                }
            }
        }
    }

    #[test]
    fn timestamps_follow_position_and_stop_while_paused() {
        let mut playback: DiscordPlayback = serde_json::from_value(serde_json::json!({
            "title": "Track", "artist": "Artist", "cover": "",
            "isPlaying": true, "isBuffering": false, "position": 35.0, "duration": 120.0, "rate": 1.0
        })).unwrap();
        assert_eq!(
            playback.timestamps(1000.0),
            Some(serde_json::json!({ "start": 965, "end": 1085 }))
        );
        playback.is_playing = false;
        assert_eq!(playback.timestamps(1010.0), None);
        playback.is_playing = true;
        playback.position = 80.0;
        playback.rate = 2.0;
        assert_eq!(
            playback.timestamps(1015.0),
            Some(serde_json::json!({ "start": 975, "end": 1035 }))
        );
    }
}
