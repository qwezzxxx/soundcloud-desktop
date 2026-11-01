use serde_json::Value;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const DISCORD_CLIENT_ID: &str = "802958833214423081"; // Official SoundCloud Discord Client ID

#[derive(Clone)]
pub struct DiscordManager {
    inner: Arc<Mutex<DiscordState>>,
}

#[cfg(target_os = "windows")]
type IpcStream = std::fs::File;

#[cfg(not(target_os = "windows"))]
type IpcStream = std::os::unix::net::UnixStream;

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
    last_title: String,
    last_artist: String,
    last_cover: String,
    last_is_playing: bool,
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
                last_title: String::new(),
                last_artist: String::new(),
                last_cover: String::new(),
                last_is_playing: false,
            })),
        };

        let mgr_clone = mgr.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(5));
                let should_reconnect = {
                    let state = mgr_clone.inner.lock().unwrap();
                    state.enabled && !state.connected
                };

                if should_reconnect {
                    let _ = mgr_clone.try_connect();
                }
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
        state.enabled = enabled;
        state.show_track = show_track;
        state.show_artist = show_artist;
        state.show_cover = show_cover;
        state.show_status = show_status;
        state.client_id = cid;

        if !enabled || cid_changed {
            state.connected = false;
            state.pipe = None;
        }

        let has_last = !state.last_title.is_empty();
        let (lt, la, lc, lip) = (state.last_title.clone(), state.last_artist.clone(), state.last_cover.clone(), state.last_is_playing);
        drop(state);

        if !enabled {
            self.clear_activity();
        } else if has_last {
            self.set_activity(&lt, &la, &lc, lip);
        }
    }

    pub fn try_connect(&self) -> bool {
        let mut state = self.inner.lock().unwrap();
        if !state.enabled {
            return false;
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
                        let _ = self.send_activity_payload(act);
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
                                let _ = self.send_activity_payload(act);
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

    pub fn set_activity(&self, title: &str, artist: &str, cover: &str, is_playing: bool) {
        let (enabled, show_track, show_artist, show_cover, show_status) = {
            let state = self.inner.lock().unwrap();
            (
                state.enabled,
                state.show_track,
                state.show_artist,
                state.show_cover,
                state.show_status,
            )
        };

        if !enabled {
            return;
        }

        let mut activity = serde_json::Map::new();

        // When show_status is true, display SoundCloud. When false, use the track name or Music
        let display_name = if show_status {
            "SoundCloud".to_string()
        } else {
            if !title.trim().is_empty() {
                title.trim().to_string()
            } else {
                "Music".to_string()
            }
        };

        activity.insert("name".to_string(), serde_json::json!(display_name));
        activity.insert("type".to_string(), serde_json::json!(2)); // 2 = Listening to

        if show_track && !title.trim().is_empty() {
            activity.insert("details".to_string(), serde_json::json!(title));
        }
        if show_artist && !artist.trim().is_empty() {
            activity.insert("state".to_string(), serde_json::json!(format!("by {}", artist)));
        }

        let mut assets = serde_json::Map::new();
        if show_cover && !cover.is_empty() {
            assets.insert("large_image".to_string(), serde_json::json!(cover));
            assets.insert("large_text".to_string(), serde_json::json!(if show_status { "SoundCloud" } else { title }));
        }

        if show_status {
            assets.insert("small_image".to_string(), serde_json::json!(if is_playing { "play" } else { "pause" }));
            assets.insert("small_text".to_string(), serde_json::json!(if is_playing { "Воспроизведение" } else { "Пауза" }));
        }

        if !assets.is_empty() {
            activity.insert("assets".to_string(), Value::Object(assets));
        }

        if is_playing {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut timestamps = serde_json::Map::new();
            timestamps.insert("start".to_string(), serde_json::json!(now));
            activity.insert("timestamps".to_string(), Value::Object(timestamps));
        }

        let act_val = Value::Object(activity);
        {
            let mut state = self.inner.lock().unwrap();
            state.current_activity = Some(act_val.clone());
            state.last_title = title.to_string();
            state.last_artist = artist.to_string();
            state.last_cover = cover.to_string();
            state.last_is_playing = is_playing;
        }

        let _ = self.send_activity_payload(act_val);
    }

    fn send_activity_payload(&self, activity: Value) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.inner.lock().unwrap();
        if !state.enabled {
            return Ok(());
        }

        if !state.connected || state.pipe.is_none() {
            drop(state);
            if !self.try_connect() {
                return Ok(());
            }
            state = self.inner.lock().unwrap();
        }

        let pid = std::process::id();
        let payload = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": pid,
                "activity": activity
            },
            "nonce": format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis())
        });

        let s = payload.to_string();
        let op = 1u32;
        let len = s.len() as u32;

        let mut packet = Vec::with_capacity(8 + s.len());
        packet.extend_from_slice(&op.to_le_bytes());
        packet.extend_from_slice(&len.to_le_bytes());
        packet.extend_from_slice(s.as_bytes());

        if let Some(pipe) = state.pipe.as_mut() {
            if pipe.write_all(&packet).is_err() || pipe.flush().is_err() {
                state.connected = false;
                state.pipe = None;
            }
        }

        Ok(())
    }

    pub fn clear_activity(&self) {
        let mut state = self.inner.lock().unwrap();
        state.current_activity = None;
        if let Some(pipe) = state.pipe.as_mut() {
            let pid = std::process::id();
            let payload = serde_json::json!({
                "cmd": "SET_ACTIVITY",
                "args": {
                    "pid": pid,
                    "activity": null
                },
                "nonce": "clear"
            });
            let s = payload.to_string();
            let op = 1u32;
            let len = s.len() as u32;
            let mut packet = Vec::with_capacity(8 + s.len());
            packet.extend_from_slice(&op.to_le_bytes());
            packet.extend_from_slice(&len.to_le_bytes());
            packet.extend_from_slice(s.as_bytes());
            let _ = pipe.write_all(&packet);
            let _ = pipe.flush();
        }
    }
}
