use reqwest::Client;
use serde_json::Value;
use crate::soundcloud::{FormattedTrack, SoundCloudClient};

pub struct MultiProviders;

impl MultiProviders {
    pub async fn search_youtube_music(http: &Client, query: &str) -> Vec<FormattedTrack> {
        let payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "ru",
                    "gl": "US"
                }
            },
            "query": query
        });

        let res = match http.post("https://music.youtube.com/youtubei/v1/search")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .json(&payload)
            .send()
            .await {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };

        let data: Value = match res.json().await {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut tracks = Vec::new();

        let sections = data.get("contents")
            .and_then(|c| c.get("tabbedSearchResultsRenderer"))
            .and_then(|t| t.get("tabs"))
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|tab| tab.get("tabRenderer"))
            .and_then(|tr| tr.get("content"))
            .and_then(|c| c.get("sectionListRenderer"))
            .and_then(|sl| sl.get("contents"))
            .and_then(|cnt| cnt.as_array())
            .cloned()
            .unwrap_or_default();

        for s in sections {
            let isr = s.get("itemSectionRenderer");
            let items = if let Some(r) = isr.and_then(|i| i.get("contents")).and_then(|c| c.as_array()) {
                r.clone()
            } else {
                continue;
            };

            for item in items {
                let item_r = match item.get("musicResponsiveListItemRenderer") {
                    Some(ir) => ir,
                    None => continue,
                };

                let flex_cols = item_r.get("flexColumns").and_then(|fc| fc.as_array());
                let mut title = String::new();
                let mut artist = String::from("YouTube Music");
                let mut duration = String::from("3:00");

                if let Some(cols) = flex_cols {
                    if let Some(col0) = cols.first() {
                        if let Some(runs) = col0.get("musicResponsiveListItemFlexColumnRenderer")
                            .and_then(|r| r.get("text"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|rn| rn.as_array()) {
                            if let Some(first_run) = runs.first().and_then(|r| r.get("text")).and_then(|t| t.as_str()) {
                                title = first_run.to_string();
                            }
                        }
                    }

                    if let Some(col1) = cols.get(1) {
                        if let Some(runs) = col1.get("musicResponsiveListItemFlexColumnRenderer")
                            .and_then(|r| r.get("text"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|rn| rn.as_array()) {
                            if let Some(first_run) = runs.first().and_then(|r| r.get("text")).and_then(|t| t.as_str()) {
                                if !first_run.is_empty() {
                                    artist = first_run.to_string();
                                }
                            }
                            for run in runs.iter().skip(1) {
                                if let Some(txt) = run.get("text").and_then(|t| t.as_str()) {
                                    if txt.contains(':') && txt.len() <= 6 {
                                        duration = txt.to_string();
                                    }
                                }
                            }
                        }
                    }
                }

                // Video ID
                let mut video_id = String::new();
                if let Some(nav) = item_r.get("navigationEndpoint") {
                    if let Some(vid) = nav.get("watchEndpoint").and_then(|w| w.get("videoId")).and_then(|v| v.as_str()) {
                        video_id = vid.to_string();
                    }
                }
                if video_id.is_empty() {
                    if let Some(overlay) = item_r.get("overlay")
                        .and_then(|o| o.get("musicItemThumbnailOverlayRenderer"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.get("musicPlayButtonRenderer"))
                        .and_then(|p| p.get("playNavigationEndpoint"))
                        .and_then(|ne| ne.get("watchEndpoint"))
                        .and_then(|we| we.get("videoId"))
                        .and_then(|v| v.as_str()) {
                        video_id = overlay.to_string();
                    }
                }

                // Cover art
                let mut cover = String::new();
                if let Some(thumbs) = item_r.get("thumbnail")
                    .and_then(|t| t.get("musicThumbnailRenderer"))
                    .and_then(|m| m.get("thumbnail"))
                    .and_then(|th| th.get("thumbnails"))
                    .and_then(|arr| arr.as_array()) {
                    if let Some(last_thumb) = thumbs.last().and_then(|th| th.get("url")).and_then(|u| u.as_str()) {
                        cover = last_thumb.to_string();
                    }
                }

                if !title.is_empty() && !video_id.is_empty() {
                    let mut duration_ms = 180_000;
                    if let Some((m, s)) = duration.split_once(':') {
                        if let (Ok(mins), Ok(secs)) = (m.parse::<u64>(), s.parse::<u64>()) {
                            duration_ms = (mins * 60 + secs) * 1000;
                        }
                    }

                    tracks.push(FormattedTrack {
                        id: Value::String(video_id.clone()),
                        title,
                        artist,
                        duration,
                        duration_ms,
                        cover: if cover.is_empty() { format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id) } else { cover },
                        liked: false,
                        description: Some("YouTube Music Audio".to_string()),
                        genre: Some("YouTube".to_string()),
                        permalink_url: Some(format!("https://music.youtube.com/watch?v={}", video_id)),
                        provider: Some("youtube".to_string()),
                        user_id: None,
                        user_permalink: None,
                        user_avatar: None,
                    });
                }
            }
        }

        tracks
    }

    pub async fn search_yandex_music(http: &Client, query: &str) -> Vec<FormattedTrack> {
        let encoded: String = query.chars().map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                let mut buf = [0; 4];
                let s = c.encode_utf8(&mut buf);
                s.bytes().map(|b| format!("%{:02X}", b)).collect()
            }
        }).collect();

        let url = format!("https://api.music.yandex.net/search?text={}&type=track&page=0", encoded);
        let res = match http.get(&url)
            .header("User-Agent", "YandexMusic/2021.08.1 (Android; Pixel 4)")
            .header("Accept", "application/json")
            .send()
            .await {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };

        let data: Value = match res.json().await {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut tracks = Vec::new();
        if let Some(items) = data.get("result")
            .and_then(|r| r.get("tracks"))
            .and_then(|t| t.get("results"))
            .and_then(|res_arr| res_arr.as_array()) {

            for t in items {
                let id = t.get("id").map(|v| {
                    if let Some(n) = v.as_i64() {
                        Value::String(n.to_string())
                    } else if let Some(s) = v.as_str() {
                        Value::String(s.to_string())
                    } else {
                        v.clone()
                    }
                }).unwrap_or(Value::Null);

                let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();

                let mut artist_names = Vec::new();
                if let Some(artists) = t.get("artists").and_then(|a| a.as_array()) {
                    for a in artists {
                        if let Some(name) = a.get("name").and_then(|n| n.as_str()) {
                            artist_names.push(name);
                        }
                    }
                }
                let artist = if artist_names.is_empty() {
                    "Yandex Music Artist".to_string()
                } else {
                    artist_names.join(", ")
                };

                let duration_ms = t.get("durationMs").and_then(|v| v.as_u64()).unwrap_or(180_000);
                let mins = duration_ms / 60000;
                let secs = (duration_ms % 60000) / 1000;
                let duration = format!("{}:{:02}", mins, secs);

                let raw_cover = t.get("ogImage")
                    .or_else(|| t.get("coverUri"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");

                let cover = if !raw_cover.is_empty() {
                    format!("https://{}", raw_cover.replace("%%", "400x400"))
                } else {
                    "https://avatars.yandex.net/get-music-content/16154377/b193de05.a.15871459-7/400x400".to_string()
                };

                let track_id_str = if let Some(s) = id.as_str() { s.to_string() } else { id.to_string() };

                tracks.push(FormattedTrack {
                    id,
                    title,
                    artist,
                    duration,
                    duration_ms,
                    cover,
                    liked: false,
                    description: Some("Яндекс Музыка HD".to_string()),
                    genre: Some("Yandex".to_string()),
                    permalink_url: Some(format!("https://music.yandex.ru/track/{}", track_id_str)),
                    provider: Some("yandex".to_string()),
                    user_id: None,
                    user_permalink: None,
                    user_avatar: None,
                });
            }
        }

        tracks
    }

    pub async fn search_spotify_catalogue(http: &Client, query: &str) -> Vec<FormattedTrack> {
        let encoded: String = query.chars().map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                let mut buf = [0; 4];
                let s = c.encode_utf8(&mut buf);
                s.bytes().map(|b| format!("%{:02X}", b)).collect()
            }
        }).collect();

        // Deezer provides full open Spotify-equivalent metadata with ISRC and high-res art without keys
        let url = format!("https://api.deezer.com/search?q={}&limit=15", encoded);
        let res = match http.get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };

        let data: Value = match res.json().await {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut tracks = Vec::new();
        if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
            for it in items {
                let id = it.get("id").map(|v| {
                    if let Some(n) = v.as_i64() {
                        Value::String(n.to_string())
                    } else if let Some(s) = v.as_str() {
                        Value::String(s.to_string())
                    } else {
                        v.clone()
                    }
                }).unwrap_or(Value::Null);

                let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                let artist = it.get("artist").and_then(|a| a.get("name")).and_then(|n| n.as_str()).unwrap_or("Spotify Artist").to_string();

                let dur_s = it.get("duration").and_then(|v| v.as_u64()).unwrap_or(180);
                let duration_ms = dur_s * 1000;
                let mins = dur_s / 60;
                let secs = dur_s % 60;
                let duration = format!("{}:{:02}", mins, secs);

                let cover = it.get("album")
                    .and_then(|a| a.get("cover_big").or_else(|| a.get("cover_medium")).or_else(|| a.get("cover")))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                tracks.push(FormattedTrack {
                    id,
                    title,
                    artist,
                    duration,
                    duration_ms,
                    cover,
                    liked: false,
                    description: Some("Spotify Catalogue Track".to_string()),
                    genre: Some("Spotify".to_string()),
                    permalink_url: it.get("link").and_then(|l| l.as_str()).map(|s| s.to_string()),
                    provider: Some("spotify".to_string()),
                    user_id: None,
                    user_permalink: None,
                    user_avatar: None,
                });
            }
        }

        tracks
    }

    pub async fn parse_spotify_link(http: &Client, url: &str) -> Vec<FormattedTrack> {
        let clean = url.trim();
        let embed_url = if clean.contains("/embed/") {
            clean.to_string()
        } else if let Some(idx) = clean.find("open.spotify.com/") {
            let path = &clean[idx + 17..];
            format!("https://open.spotify.com/embed/{}", path)
        } else {
            clean.to_string()
        };

        let res = match http.get(&embed_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .send()
            .await {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };

        let html = match res.text().await {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let marker = r#"<script id="__NEXT_DATA__" type="application/json">"#;
        let start = match html.find(marker) {
            Some(s) => s + marker.len(),
            None => return Vec::new(),
        };
        let end = match html[start..].find("</script>") {
            Some(e) => start + e,
            None => return Vec::new(),
        };

        let json_str = &html[start..end];
        let data: Value = match serde_json::from_str(json_str) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let entity = match data.get("props")
            .and_then(|p| p.get("pageProps"))
            .and_then(|pp| pp.get("state"))
            .and_then(|s| s.get("data"))
            .and_then(|d| d.get("entity")) {
                Some(e) => e,
                None => return Vec::new(),
            };

        let mut tracks = Vec::new();

        // 1. If playlist or album with trackList
        if let Some(track_list) = entity.get("trackList").and_then(|tl| tl.as_array()) {
            for (idx, item) in track_list.iter().enumerate() {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("Spotify Track").to_string();
                let artist = item.get("subtitle").and_then(|s| s.as_str()).unwrap_or("Spotify Artist").to_string();
                let duration_ms = item.get("duration").and_then(|d| d.as_u64()).unwrap_or(180_000);
                let mins = duration_ms / 60000;
                let secs = (duration_ms % 60000) / 1000;
                let duration = format!("{}:{:02}", mins, secs);

                let id = item.get("uri").and_then(|u| u.as_str())
                    .or_else(|| item.get("id").and_then(|i| i.as_str()))
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or_else(|| Value::String(format!("spotify_{}", idx)));

                tracks.push(FormattedTrack {
                    id,
                    title,
                    artist,
                    duration,
                    duration_ms,
                    cover: "https://open.spotify.com/favicon.ico".to_string(),
                    liked: false,
                    description: Some("Spotify Imported Track".to_string()),
                    genre: Some("Spotify".to_string()),
                    permalink_url: Some(clean.to_string()),
                    provider: Some("spotify".to_string()),
                    user_id: None,
                    user_permalink: None,
                    user_avatar: None,
                });
            }
        } else {
            // 2. Single track
            let title = entity.get("name").and_then(|n| n.as_str()).unwrap_or("Spotify Track").to_string();
            let mut artists = Vec::new();
            if let Some(arr) = entity.get("artists").and_then(|a| a.as_array()) {
                for a in arr {
                    if let Some(name) = a.get("name").and_then(|n| n.as_str()) {
                        artists.push(name);
                    }
                }
            }
            let artist = if artists.is_empty() { "Spotify Artist".to_string() } else { artists.join(", ") };
            let duration_ms = entity.get("duration").and_then(|d| d.as_u64()).unwrap_or(180_000);
            let mins = duration_ms / 60000;
            let secs = (duration_ms % 60000) / 1000;
            let duration = format!("{}:{:02}", mins, secs);

            tracks.push(FormattedTrack {
                id: Value::String(clean.to_string()),
                title,
                artist,
                duration,
                duration_ms,
                cover: "https://open.spotify.com/favicon.ico".to_string(),
                liked: false,
                description: Some("Spotify Track".to_string()),
                genre: Some("Spotify".to_string()),
                permalink_url: Some(clean.to_string()),
                provider: Some("spotify".to_string()),
                user_id: None,
                user_permalink: None,
                user_avatar: None,
            });
        }

        tracks
    }

    pub async fn search_unified(
        http: &Client,
        sc: &SoundCloudClient,
        query: &str,
        provider: Option<&str>,
    ) -> Vec<FormattedTrack> {
        let clean = query.trim();

        // 1. Direct link detection
        if clean.contains("spotify.com/") {
            return Self::parse_spotify_link(http, clean).await;
        }

        match provider.unwrap_or("all") {
            "youtube" => Self::search_youtube_music(http, clean).await,
            "yandex" => Self::search_yandex_music(http, clean).await,
            "spotify" => Self::search_spotify_catalogue(http, clean).await,
            "soundcloud" => {
                let mut sc_tracks = sc.search_tracks(clean).await.unwrap_or_default();
                for t in &mut sc_tracks {
                    if t.provider.is_none() {
                        t.provider = Some("soundcloud".to_string());
                    }
                }
                sc_tracks
            },
            _ => {
                // "all" - Search SoundCloud, YouTube Music, and Yandex Music in parallel!
                let (sc_res, yt_res, ym_res) = tokio::join!(
                    sc.search_tracks(clean),
                    Self::search_youtube_music(http, clean),
                    Self::search_yandex_music(http, clean)
                );

                let mut unified = Vec::new();
                let mut sc_tracks = sc_res.unwrap_or_default();
                for t in &mut sc_tracks {
                    t.provider = Some("soundcloud".to_string());
                }

                let max_len = sc_tracks.len().max(yt_res.len()).max(ym_res.len());
                for i in 0..max_len {
                    if i < sc_tracks.len() {
                        unified.push(sc_tracks[i].clone());
                    }
                    if i < yt_res.len() {
                        unified.push(yt_res[i].clone());
                    }
                    if i < ym_res.len() {
                        unified.push(ym_res[i].clone());
                    }
                }

                unified
            }
        }
    }
}
