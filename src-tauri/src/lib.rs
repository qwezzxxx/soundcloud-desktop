mod auth_server;
mod discord;
mod soundcloud;
mod multi_providers;
use multi_providers::MultiProviders;

use discord::DiscordManager;
use serde_json::Value;
use soundcloud::{AppConfig, FormattedTrack, SoundCloudClient};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub sc: Arc<SoundCloudClient>,
    pub discord: DiscordManager,
}

#[tauri::command]
async fn init_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Value, String> {
    let _ = std::fs::write("init_app_called.log", "init_app was called!\n");
    let cfg = state.sc.load_config();
    let mut likes = state.sc.load_local_likes();
    if likes.is_empty() {
        likes = cfg.cached_likes.clone().unwrap_or_default();
    }
    let profile = cfg.user_profile.clone();
    let mixed = cfg.cached_mixed.clone();
    let user = profile.clone();
    let has_token = cfg.oauth_token.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false);

    // Configure initial Discord options
    let discord_enabled = cfg.discord_rpc.unwrap_or(true);
    let show_track = cfg.discord_show_track.unwrap_or(true);
    let show_artist = cfg.discord_show_artist.unwrap_or(true);
    let show_cover = cfg.discord_show_cover.unwrap_or(true);
    let show_status = cfg.discord_show_status.unwrap_or(true);
    let discord_client_id = cfg.discord_client_id.clone().unwrap_or_default();

    state.discord.set_options(
        discord_enabled,
        show_track,
        show_artist,
        show_cover,
        show_status,
        discord_client_id,
    );

    // Background update without blocking startup
    if has_token {
        let sc_clone = state.sc.clone();
        let app_handle = app.clone();
        tokio::spawn(async move {
            let _ = sc_clone.get_profile().await;
            if let Ok(fetched) = sc_clone.get_liked_tracks(300).await {
                if !fetched.is_empty() {
                    let _ = sc_clone.save_local_likes(&fetched);
                    use tauri::Emitter;
                    let _ = app_handle.emit("likes_updated", &fetched);
                }
            }
            let _ = sc_clone.get_mixed_selections().await;
        });
    }

    Ok(serde_json::json!({
        "config": cfg,
        "likes": likes,
        "user": user,
        "profile": profile,
        "hasToken": has_token,
        "mixed": mixed
    }))
}

#[tauri::command]
async fn save_local_likes(likes: Vec<Value>, state: State<'_, AppState>) -> Result<(), String> {
    let mut tracks = Vec::new();
    for v in likes {
        if let Ok(t) = serde_json::from_value::<FormattedTrack>(v) {
            tracks.push(t);
        }
    }
    state.sc.save_local_likes(&tracks)
}

#[tauri::command]
async fn get_local_likes(state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    Ok(state.sc.load_local_likes())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.sc.load_config())
}

#[tauri::command]
async fn login_account(
    identifier: String,
    token: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    state.sc.login_account(identifier, token).await
}

#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.sc.load_config();
    cfg.oauth_token = Some("".to_string());
    cfg.user_id = None;
    cfg.user_profile = None;
    cfg.cached_likes = Some(Vec::new());
    cfg.cached_mixed = None;
    cfg.session = None;
    cfg.last_tab = Some("tab-home".to_string());
    let _ = state.sc.save_raw_config(&cfg);
    state.discord.clear_activity();
    Ok(())
}

#[tauri::command]
async fn save_config(cfg: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    let discord_enabled = cfg.discord_rpc.unwrap_or(true);
    let show_track = cfg.discord_show_track.unwrap_or(true);
    let show_artist = cfg.discord_show_artist.unwrap_or(true);
    let show_cover = cfg.discord_show_cover.unwrap_or(true);
    let show_status = cfg.discord_show_status.unwrap_or(true);
    let discord_client_id = cfg.discord_client_id.clone().unwrap_or_default();

    state.discord.set_options(
        discord_enabled,
        show_track,
        show_artist,
        show_cover,
        show_status,
        discord_client_id,
    );
    if !discord_enabled {
        state.discord.clear_activity();
    }

    state.sc.save_config(&cfg)
}

#[tauri::command]
async fn get_profile(state: State<'_, AppState>) -> Result<Value, String> {
    state.sc.get_profile().await
}

#[tauri::command]
async fn get_likes(limit: Option<usize>, state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    state.sc.get_liked_tracks(limit.unwrap_or(500)).await
}

#[tauri::command]
async fn sync_likes(state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    state.sc.get_liked_tracks(500).await
}

#[tauri::command]
async fn push_local_likes(state: State<'_, AppState>) -> Result<usize, String> {
    state.sc.push_local_likes_to_soundcloud().await
}

#[tauri::command]
async fn get_lyrics(title: String, artist: String, duration: Option<f64>, state: State<'_, AppState>) -> Result<Value, String> {
    state.sc.fetch_lyrics(title, artist, duration).await
}

#[tauri::command]
fn log_debug(msg: String) {
    use std::io::Write;
    let p = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("debug_play.log"))).unwrap_or_else(|| std::path::PathBuf::from("debug_play.log"));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "{}", msg);
    }
}

#[tauri::command]
async fn get_stream_url(track_id: Value, state: State<'_, AppState>) -> Result<String, String> {
    use std::io::Write;
    let p = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("debug_play.log"))).unwrap_or_else(|| std::path::PathBuf::from("debug_play.log"));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "[Rust] get_stream_url called with track_id={:?}", track_id);
    }
    let res = state.sc.resolve_stream_url(track_id).await;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "[Rust] get_stream_url resolved: {:?}", res);
    }
    res
}

#[tauri::command]
async fn toggle_like(track_id: Value, should_like: bool, state: State<'_, AppState>) -> Result<bool, String> {
    state.sc.toggle_like(track_id, should_like).await
}

#[tauri::command]
async fn search_tracks(query: String, provider: Option<String>, state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    let clean = query.trim();
    if clean.is_empty() {
        return Ok(Vec::new());
    }
    let prov = provider.as_deref().unwrap_or("all");
    let tracks = MultiProviders::search_unified(state.sc.http_client(), &state.sc, clean, Some(prov)).await;
    Ok(tracks)
}

#[tauri::command]
async fn resolve_tracks(ids: Vec<Value>, state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    state.sc.resolve_tracks(ids).await
}

#[tauri::command]
async fn get_mixed_selections(state: State<'_, AppState>) -> Result<Value, String> {
    state.sc.get_mixed_selections().await
}

#[tauri::command]
async fn get_system_playlist(urn: String, state: State<'_, AppState>) -> Result<Value, String> {
    state.sc.get_system_playlist(&urn).await
}

#[tauri::command]
async fn get_related_tracks(track_id: Value, state: State<'_, AppState>) -> Result<Vec<FormattedTrack>, String> {
    state.sc.get_related_tracks(track_id).await
}

#[tauri::command]
fn open_browser_login() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "http://127.0.0.1:49221/login"])
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg("http://127.0.0.1:49221/login")
            .spawn();
    }
    Ok(())
}

#[tauri::command]
async fn set_oauth_token(token: String, state: State<'_, AppState>) -> Result<Value, String> {
    state.sc.set_oauth_token(&token).await
}

#[tauri::command]
async fn open_in_app_login(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri::Manager;
    if let Some(existing) = app.get_webview_window("soundcloud_login") {
        let _ = existing.set_focus();
        return Ok(());
    }

    let auth_script = r#"
        (function() {
            if (window.__ONYX_AUTH_INJECTED__) return;
            window.__ONYX_AUTH_INJECTED__ = true;

            var tokenFound = false;

            function sendToken(tok) {
                if (tokenFound || !tok) return;
                tok = decodeURIComponent(tok).trim();
                if (tok.length < 10) return;
                tokenFound = true;

                try {
                    var banner = document.createElement('div');
                    banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999999;background:#ff5500;color:#fff;text-align:center;padding:12px;font-family:sans-serif;font-size:13px;font-weight:bold;box-shadow:0 4px 16px rgba(0,0,0,0.5);';
                    banner.innerText = '✨ Авторизация успешна! Подключаем аккаунт к ONYX...';
                    document.body.appendChild(banner);
                } catch(e) {}

                try {
                    var img = new Image();
                    img.src = 'http://127.0.0.1:49221/auth?token=' + encodeURIComponent(tok);
                } catch(e) {}

                try {
                    fetch('http://127.0.0.1:49221/auth?token=' + encodeURIComponent(tok), { mode: 'no-cors' }).catch(function(){});
                } catch(e) {}

                setTimeout(function() {
                    window.location.href = 'https://soundcloud.com/?onyx_oauth_token=' + encodeURIComponent(tok);
                }, 300);
            }

            function check() {
                if (tokenFound) return;
                try {
                    var m = document.cookie.match(/oauth_token=([^;]+)/);
                    if (m && m[1]) {
                        sendToken(m[1]);
                        return;
                    }
                } catch(e) {}

                try {
                    for (var i = 0; i < localStorage.length; i++) {
                        var k = localStorage.key(i);
                        if (k === 'oauth_token' || k === 'sc_oauth_token') {
                            var v = localStorage.getItem(k);
                            if (v && v.length > 10) {
                                sendToken(v);
                                return;
                            }
                        }
                    }
                } catch(e) {}
            }

            try {
                var originalFetch = window.fetch;
                window.fetch = function(input, init) {
                    try {
                        if (init && init.headers) {
                            var auth = null;
                            if (init.headers instanceof Headers) {
                                auth = init.headers.get('Authorization') || init.headers.get('authorization');
                            } else if (typeof init.headers === 'object') {
                                auth = init.headers['Authorization'] || init.headers['authorization'];
                            }
                            if (auth && typeof auth === 'string') {
                                var mAuth = auth.match(/OAuth\s+([^\s;]+)/i) || auth.match(/Bearer\s+([^\s;]+)/i);
                                if (mAuth && mAuth[1]) {
                                    sendToken(mAuth[1]);
                                }
                            }
                        }
                    } catch(e) {}
                    return originalFetch.apply(this, arguments);
                };
            } catch(e) {}

            try {
                var origSetHeader = XMLHttpRequest.prototype.setRequestHeader;
                XMLHttpRequest.prototype.setRequestHeader = function(header, value) {
                    try {
                        if (header && header.toLowerCase() === 'authorization' && value) {
                            var mAuth = value.match(/OAuth\s+([^\s;]+)/i) || value.match(/Bearer\s+([^\s;]+)/i);
                            if (mAuth && mAuth[1]) {
                                sendToken(mAuth[1]);
                            }
                        }
                    } catch(e) {}
                    return origSetHeader.apply(this, arguments);
                };
            } catch(e) {}

            setInterval(check, 600);
            check();
        })();
    "#;

    let target_url: tauri::Url = "https://soundcloud.com/signin".parse().unwrap();
    let app_handle = app.clone();
    let sc_clone = state.sc.clone();

    let mut builder = tauri::WebviewWindowBuilder::new(
        &app,
        "soundcloud_login",
        tauri::WebviewUrl::External(target_url)
    )
    .title("Вход в SoundCloud — ONYX")
    .inner_size(540.0, 720.0)
    .center()
    .initialization_script(auth_script);

    let app_nav = app_handle.clone();
    let sc_nav = sc_clone.clone();

    builder = builder.on_navigation(move |url| {
        let url_str = url.as_str();
        if url_str.contains("onyx_oauth_token=") {
            let token = if let Some(idx) = url_str.find("onyx_oauth_token=") {
                let rest = &url_str[idx + 17..];
                let end = rest.find('&').unwrap_or(rest.len());
                let clean = &rest[..end];
                urlencoding::decode(clean).unwrap_or_else(|_| clean.into()).to_string()
            } else {
                String::new()
            };

            let trimmed = token.trim().trim_matches(&['"', '\''][..]).trim();
            if !trimmed.is_empty() {
                let app_inner = app_nav.clone();
                let sc_inner = sc_nav.clone();
                let tok_clone = trimmed.to_string();
                tokio::spawn(async move {
                    // 1. Immediately persist token so it is NEVER lost
                    {
                        let mut cfg = sc_inner.load_config();
                        cfg.oauth_token = Some(tok_clone.clone());
                        let _ = sc_inner.save_raw_config(&cfg);
                    }

                    // 2. Hide webview immediately so UI feels instant
                    if let Some(win) = app_inner.get_webview_window("soundcloud_login") {
                        let _ = win.hide();
                    }

                    // 3. Process full account authorization (profile, likes, sync)
                    match sc_inner.set_oauth_token(&tok_clone).await {
                        Ok(profile_data) => {
                            let fresh_likes = sc_inner.get_liked_tracks(500).await.unwrap_or_default();
                            let mut res_data = profile_data;
                            if let Some(obj) = res_data.as_object_mut() {
                                if let Ok(val) = serde_json::to_value(&fresh_likes) {
                                    obj.insert("likes".to_string(), val);
                                }
                            }
                            use tauri::Emitter;
                            let _ = app_inner.emit("sc_auth_success", &res_data);
                        }
                        Err(e) => {
                            eprintln!("set_oauth_token error: {}", e);
                            let cfg = sc_inner.load_config();
                            let profile = cfg.user_profile.clone();
                            let likes = cfg.cached_likes.clone().unwrap_or_default();
                            let fallback = serde_json::json!({
                                "profile": profile,
                                "likes": likes,
                                "config": cfg
                            });
                            use tauri::Emitter;
                            let _ = app_inner.emit("sc_auth_success", &fallback);
                        }
                    }

                    // 4. Safely destroy webview without closing application
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    if let Some(win) = app_inner.get_webview_window("soundcloud_login") {
                        let _ = win.close();
                    }
                });
                return false;
            }
        }
        true
    });

    let _win = builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn close_login_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("soundcloud_login") {
        let _ = win.hide();
        let _ = win.close();
    }
    Ok(())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    let clean = url.trim();
    if clean.starts_with("http://") || clean.starts_with("https://") {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", clean])
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(clean)
                .spawn();
        }
    }
    Ok(())
}

#[tauri::command]
fn update_discord_activity(
    title: String,
    artist: String,
    cover: String,
    is_playing: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.discord.set_activity(&title, &artist, &cover, is_playing);
    Ok(())
}

#[tauri::command]
fn clear_discord_activity(state: State<'_, AppState>) -> Result<(), String> {
    state.discord.clear_activity();
    Ok(())
}

#[tauri::command]
fn win_start_drag(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
fn win_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn win_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn win_close(window: tauri::Window) -> Result<(), String> {
    // Background mode: hide window to tray / background so music continues playing
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn win_exit(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
fn win_show(window: tauri::Window) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_custom_wallpaper(data: String) -> Result<(), String> {
    let mut path = PathBuf::from("custom_wallpaper.dat");
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            path = parent.join("custom_wallpaper.dat");
        }
    }
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_custom_wallpaper() -> Result<String, String> {
    let mut path = PathBuf::from("custom_wallpaper.dat");
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            path = parent.join("custom_wallpaper.dat");
        }
    }
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn clear_custom_wallpaper() -> Result<(), String> {
    let mut path = PathBuf::from("custom_wallpaper.dat");
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            path = parent.join("custom_wallpaper.dat");
        }
    }
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

fn resolve_config_path() -> PathBuf {
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            let candidate = parent.join("config.json");
            if candidate.exists() {
                if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&candidate) {
                    drop(f);
                    return candidate;
                }
            } else {
                let test_p = parent.join(".sc_write_test");
                if let Ok(_) = std::fs::write(&test_p, "1") {
                    let _ = std::fs::remove_file(&test_p);
                    return candidate;
                }
            }
        }
    }
    let config_dir = std::env::var("APPDATA")
        .or_else(|_| std::env::var("LOCALAPPDATA"))
        .or_else(|_| std::env::var("XDG_CONFIG_HOME"))
        .or_else(|_| std::env::var("HOME").map(|h| format!("{}/.config", h)))
        .unwrap_or_else(|_| ".".to_string());
    let onyx_dir = PathBuf::from(&config_dir).join("com.onyx.player");
    let legacy_dir = PathBuf::from(&config_dir).join("com.soundcloud.desktop");
    if !onyx_dir.exists() && legacy_dir.exists() {
        let _ = std::fs::create_dir_all(&onyx_dir);
        let _ = std::fs::copy(legacy_dir.join("config.json"), onyx_dir.join("config.json"));
        let _ = std::fs::copy(legacy_dir.join("likes.json"), onyx_dir.join("likes.json"));
    }
    let _ = std::fs::create_dir_all(&onyx_dir);
    onyx_dir.join("config.json")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let _ = std::fs::write("app_run.log", "lib::run() started\n");
    let config_path = resolve_config_path();

    let sc = Arc::new(SoundCloudClient::new(config_path));
    let discord = DiscordManager::new();

    // Start background local auth server on 127.0.0.1:49221
    auth_server::start_auth_server(sc.clone());

    let cfg = sc.load_config();
    let theme_name = cfg.theme.as_deref().unwrap_or("wine");
    let bg_color = match theme_name {
        "light" => tauri::utils::config::Color(248, 250, 252, 255), // #f8fafc Clean Light
        "oled" => tauri::utils::config::Color(0, 0, 0, 255),       // #000000 OLED
        "cyan" => tauri::utils::config::Color(3, 4, 9, 255),       // #030409 Cyber Neon
        "emerald" => tauri::utils::config::Color(6, 9, 8, 255),    // #060908 Emerald
        "amethyst" => tauri::utils::config::Color(8, 6, 15, 255),  // #08060f Amethyst
        "sunset" => tauri::utils::config::Color(10, 7, 10, 255),   // #0a070a Sunset
        "midnight" => tauri::utils::config::Color(5, 7, 17, 255),  // #050711 Midnight
        "flame" => tauri::utils::config::Color(9, 8, 12, 255),     // #09080c Flame
        _ => tauri::utils::config::Color(8, 7, 13, 255),          // #08070d Wine
    };

    let state = AppState { sc, discord };

    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(bg_color));
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
                let win_clone = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(350));
                    let _ = win_clone.show();
                });
            }

            // System Tray setup for background running & tray restore
            use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
            use tauri::menu::{Menu, MenuItem};

            let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Открыть ONYX", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            if let Some(icon) = app.default_window_icon() {
                let _ = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .tooltip("ONYX")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "quit" => {
                                app.exit(0);
                            }
                            "show" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.unminimize();
                                    let _ = w.set_focus();
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                if let Ok(is_visible) = w.is_visible() {
                                    if is_visible {
                                        let _ = w.hide();
                                    } else {
                                        let _ = w.show();
                                        let _ = w.unminimize();
                                        let _ = w.set_focus();
                                    }
                                } else {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                    })
                    .build(app);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login_account,
            logout,
            init_app,
            save_local_likes,
            get_local_likes,
            get_config,
            save_config,
            get_profile,
            get_likes,
            sync_likes,
            push_local_likes,
            get_stream_url,
            toggle_like,
            search_tracks,
            resolve_tracks,
            get_mixed_selections,
            get_system_playlist,
            get_related_tracks,
            open_browser_login,
            open_in_app_login,
            set_oauth_token,
            close_login_window,
            open_url,
            update_discord_activity,
            clear_discord_activity,
            win_start_drag,
            win_minimize,
            win_toggle_maximize,
            win_close,
            win_exit,
            win_show,
            log_debug,
            get_lyrics,
            save_custom_wallpaper,
            load_custom_wallpaper,
            clear_custom_wallpaper
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Prevent launcher from quitting when windows (like login modal or main hide) are closed
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
