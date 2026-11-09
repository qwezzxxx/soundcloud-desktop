use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

mod ipc;
use ipc::Presence;

pub const DISCORD_CLIENT_ID: &str = "802958833214423081"; // Official SoundCloud Discord Client ID

#[derive(Clone)]
pub struct DiscordManager {
    inner: Arc<Mutex<DiscordState>>,
    desired: watch::Sender<Option<Presence>>,
}

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
    enabled: bool,
    client_id: String,
    show_track: bool,
    show_artist: bool,
    show_cover: bool,
    show_status: bool,
    last_playback: Option<(DiscordPlayback, f64)>,
}

impl DiscordManager {
    pub fn new() -> Self {
        let (desired, updates) = watch::channel(None);
        tauri::async_runtime::spawn(ipc::run(updates));
        Self {
            inner: Arc::new(Mutex::new(DiscordState {
                enabled: true,
                client_id: DISCORD_CLIENT_ID.to_string(),
                show_track: true,
                show_artist: true,
                show_cover: true,
                show_status: true,
                last_playback: None,
            })),
            desired,
        }
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

        self.publish(&state);
    }

    fn publish(&self, state: &DiscordState) {
        let latest = state.enabled.then(|| Presence {
            client_id: state.client_id.clone(),
            activity: state
                .last_playback
                .as_ref()
                .map(|(playback, now)| Self::render_activity(state, playback, *now))
                .unwrap_or(Value::Null),
        });
        self.desired.send_if_modified(|desired| {
            if *desired == latest {
                return false;
            }
            *desired = latest;
            true
        });
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
        let paused = !is_playing && !playback.is_buffering;
        let mut activity = serde_json::json!({
            "name": if state.show_status { "ONYX SOUND" } else if !state.show_track || title.trim().is_empty() { "Music" } else { title.trim() },
            "type": 2,
            "status_display_type": 0
        });

        if state.show_track && !title.trim().is_empty() {
            activity["details"] = serde_json::json!(title);
        }
        let mut status = Vec::new();
        if paused {
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
        // Mirror the player's button independently of the activity name.
        assets.insert(
            "small_image".to_string(),
            serde_json::json!(if paused { "play" } else { "pause" }),
        );
        assets.insert(
            "small_text".to_string(),
            serde_json::json!(if paused {
                "На паузе"
            } else if playback.is_buffering {
                "Буферизация"
            } else {
                "Воспроизведение"
            }),
        );
        activity["assets"] = Value::Object(assets);

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
        let mut state = self.inner.lock().unwrap();
        state.last_playback = Some((playback, now));
        self.publish(&state);
    }

    pub fn clear_activity(&self) {
        let mut state = self.inner.lock().unwrap();
        state.last_playback = None;
        self.publish(&state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_pause_in_the_card_with_the_status_badge_disabled() {
        let state = DiscordState {
            enabled: true,
            client_id: DISCORD_CLIENT_ID.to_string(),
            show_track: true,
            show_artist: true,
            show_cover: true,
            show_status: false,
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
        assert_eq!(paused["assets"]["small_image"], "play");
        assert!(paused.get("timestamps").is_none());

        playback.is_buffering = true;
        let buffering = DiscordManager::render_activity(&state, &playback, 1000.0);
        assert_eq!(buffering["state"], "by Artist");

        playback.is_buffering = false;
        playback.is_playing = true;
        let resumed = DiscordManager::render_activity(&state, &playback, 1000.0);
        assert_eq!(resumed["state"], "by Artist");
        assert_eq!(resumed["assets"]["small_image"], "pause");
        assert!(resumed.get("timestamps").is_some());
    }

    #[test]
    fn branding_toggle_preserves_track_details_and_playback_action() {
        for show_status in [false, true] {
            for is_playing in [false, true] {
                for show_track in [false, true] {
                    let state = DiscordState {
                        enabled: true,
                        client_id: DISCORD_CLIENT_ID.to_string(),
                        show_track, show_artist: true, show_cover: true, show_status,
                        last_playback: None,
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
                        assert_eq!(activity["assets"]["small_image"], "play");
                        assert_eq!(activity["assets"]["small_text"], "На паузе");
                    } else {
                        assert_eq!(activity["assets"]["small_image"], "pause");
                        assert_eq!(activity["assets"]["small_text"], "Воспроизведение");
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
