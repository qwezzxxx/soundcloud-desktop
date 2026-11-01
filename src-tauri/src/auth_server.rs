use std::sync::{Arc, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use tokio::net::TcpListener;
use crate::soundcloud::SoundCloudClient;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_default()
    })
}

pub fn start_auth_server(sc: Arc<SoundCloudClient>) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Auth server runtime init error: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            let listener = match TcpListener::bind("127.0.0.1:49221").await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Auth server bind failed: {}", e);
                    return;
                }
            };

            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let sc_ref = sc.clone();
                    tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    if let Ok(bytes_read) = stream.read(&mut buffer).await {
                        if bytes_read == 0 {
                            return;
                        }
                        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                        let first_line = request_str.lines().next().unwrap_or("");
                        let parts: Vec<&str> = first_line.split_whitespace().collect();
                        if parts.len() < 2 {
                            return;
                        }

                        let method = parts[0];
                        let path = parts[1];

                        if method == "OPTIONS" {
                            let response = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, OPTIONS, HEAD\r\nAccess-Control-Allow-Headers: Range, Origin, Content-Type, Accept\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes()).await;
                            return;
                        }

                        if path.starts_with("/file") {
                            let file_name = if let Some(idx) = path.find("name=") {
                                let raw = &path[idx + 5..];
                                let end = raw.find('&').unwrap_or(raw.len());
                                urldecode(&raw[..end])
                            } else {
                                String::new()
                            };

                            let clean_name: String = file_name.chars()
                                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                                .collect();

                            let cache_dir = std::env::current_exe()
                                .ok()
                                .and_then(|p| p.parent().map(|d| d.join("audio_cache")))
                                .unwrap_or_else(|| std::path::PathBuf::from("audio_cache"));

                            let file_path = cache_dir.join(&clean_name);

                            if file_path.exists() {
                                if let Ok(mut f) = tokio::fs::File::open(&file_path).await {
                                    let metadata = f.metadata().await;
                                    let file_size = metadata.map(|m| m.len()).unwrap_or(0);

                                    let range_header = request_str.lines()
                                        .find(|l| l.to_lowercase().starts_with("range:"))
                                        .and_then(|l| l.split(':').nth(1))
                                        .map(|s| s.trim().to_string());

                                    let (start, end) = if let Some(ref r) = range_header {
                                        parse_range_header(r, file_size)
                                    } else {
                                        (0, file_size.saturating_sub(1))
                                    };

                                    let length = if file_size > 0 && end >= start {
                                        (end - start) + 1
                                    } else {
                                        0
                                    };

                                    let _ = f.seek(std::io::SeekFrom::Start(start)).await;

                                    let status_line = if range_header.is_some() {
                                        "HTTP/1.1 206 Partial Content\r\n"
                                    } else {
                                        "HTTP/1.1 200 OK\r\n"
                                    };

                                    let content_type = if clean_name.ends_with(".mp3") {
                                        "audio/mpeg"
                                    } else {
                                        "audio/mp4"
                                    };

                                    let header_str = format!(
                                        "{}Access-Control-Allow-Origin: *\r\nAccept-Ranges: bytes\r\nContent-Type: {}\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                                        status_line,
                                        content_type,
                                        length,
                                        start,
                                        end,
                                        file_size
                                    );

                                    if stream.write_all(header_str.as_bytes()).await.is_ok() {
                                        let mut chunk_buf = [0u8; 65536];
                                        let mut remaining = length;
                                        while remaining > 0 {
                                            let to_read = std::cmp::min(remaining, chunk_buf.len() as u64) as usize;
                                            if let Ok(n) = f.read(&mut chunk_buf[..to_read]).await {
                                                if n == 0 { break; }
                                                if stream.write_all(&chunk_buf[..n]).await.is_err() { break; }
                                                remaining -= n as u64;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    return;
                                }
                            }
                            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r
").await;
                            return;
                        }

                        if path.starts_with("/stream") {
                            let target_url = if let Some(idx) = path.find("url=") {
                                let raw = &path[idx + 4..];
                                urldecode(raw)
                            } else {
                                String::new()
                            };

                            if target_url.is_empty() {
                                let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n").await;
                                return;
                            }

                            let client = get_http_client();

                            // Extract Range header if requested by HTML5 audio
                            let range_header = request_str.lines()
                                .find(|l| l.to_lowercase().starts_with("range:"))
                                .and_then(|l| l.split(':').nth(1))
                                .map(|s| s.trim().to_string());

                            let mut req = client.get(&target_url)
                                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36")
                                ;

                            if let Some(ref r) = range_header {
                                req = req.header("Range", r);
                            }

                            if let Ok(mut resp) = req.send().await {
                                let status = resp.status();
                                let content_type = resp.headers().get("content-type")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("audio/mp4");
                                let content_len = resp.headers().get("content-length")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("");
                                let content_range = resp.headers().get("content-range")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("");

                                let status_line = if status.is_success() && status.as_u16() == 206 {
                                    "HTTP/1.1 206 Partial Content\r\n"
                                } else {
                                    "HTTP/1.1 200 OK\r\n"
                                };

                                let mut header_str = format!(
                                    "{}Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, OPTIONS, HEAD\r\nAccess-Control-Allow-Headers: Range, Origin, Content-Type, Accept\r\nAccept-Ranges: bytes\r\nConnection: keep-alive\r\nContent-Type: {}\r\n",
                                    status_line,
                                    content_type
                                );
                                if !content_len.is_empty() {
                                    header_str.push_str(&format!("Content-Length: {}\r\n", content_len));
                                }
                                if !content_range.is_empty() {
                                    header_str.push_str(&format!("Content-Range: {}\r\n", content_range));
                                }
                                header_str.push_str("\r\n");

                                if stream.write_all(header_str.as_bytes()).await.is_ok() {
                                    while let Ok(Some(chunk)) = resp.chunk().await {
                                        if stream.write_all(&chunk).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            return;
                        }

                        if path.starts_with("/auth") {
                            // Extract token parameter
                            let token_val = if let Some(idx) = path.find("token=") {
                                let query = &path[idx + 6..];
                                let raw_token = query.split('&').next().unwrap_or("");
                                urldecode(raw_token)
                            } else {
                                String::new()
                            };

                            let trimmed = token_val.trim();
                            if !trimmed.is_empty() {
                                let mut cfg = sc_ref.load_config();
                                cfg.oauth_token = Some(trimmed.to_string());
                                let _ = sc_ref.save_config(&cfg);

                                // Refresh profile
                                let profile = sc_ref.get_profile().await.ok();
                                let username = profile
                                    .as_ref()
                                    .and_then(|p| p.get("username"))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or("SoundCloud User");

                                let body = format!(
                                    r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>SoundCloud Player Connected</title>
    <style>
        body {{ background: #0c0a10; color: #fff; font-family: -apple-system, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
        .card {{ background: #161420; border: 1px solid rgba(255,255,255,0.1); border-radius: 20px; padding: 40px; text-align: center; max-width: 440px; box-shadow: 0 20px 40px rgba(0,0,0,0.6); }}
        h1 {{ font-size: 20px; margin-bottom: 8px; color: #ff5500; }}
        p {{ font-size: 13px; color: #aaa; line-height: 1.5; }}
        .user {{ display: inline-block; background: rgba(255,85,0,0.15); border: 1px solid rgba(255,85,0,0.3); padding: 6px 14px; border-radius: 9999px; font-weight: bold; color: #ff7700; margin: 15px 0; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>✅ Успешно подключено!</h1>
        <div class="user">👤 {}</div>
        <p>Аккаунт успешно привязан к десктопному плееру. Все лайки и миксы уже синхронизированы в приложении.</p>
        <p style="font-size:11px; color:#666; margin-top:20px;">Можете закрыть эту вкладку и вернуться в плеер.</p>
    </div>
</body>
</html>"#,
                                    username
                                );

                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = stream.write_all(response.as_bytes()).await;
                            } else {
                                let body = "Missing token";
                                let response = format!(
                                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = stream.write_all(response.as_bytes()).await;
                            }
                        } else {
                            // Serve login helper page
                            let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Вход в SoundCloud Player</title>
    <style>
        body { background: #0c0a10; color: #fff; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 20px; box-sizing: border-box; }
        .card { background: #15131e; border: 1px solid rgba(255,255,255,0.1); border-radius: 24px; padding: 36px; max-width: 520px; width: 100%; box-shadow: 0 25px 50px -12px rgba(0,0,0,0.7); }
        .badge { display: inline-flex; align-items: center; gap: 6px; background: rgba(255,85,0,0.12); color: #ff5500; font-size: 11px; font-weight: bold; padding: 4px 10px; border-radius: 9999px; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 16px; border: 1px solid rgba(255,85,0,0.25); }
        h1 { font-size: 22px; font-weight: 800; margin: 0 0 10px 0; color: #fff; }
        p { font-size: 13px; color: #9ca3af; line-height: 1.6; margin: 0 0 20px 0; }
        .steps { display: flex; flex-direction: column; gap: 14px; margin-bottom: 24px; }
        .step { background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 14px; padding: 14px 16px; }
        .step-num { font-size: 11px; font-weight: bold; color: #ff5500; margin-bottom: 4px; }
        .step-title { font-size: 13px; font-weight: 600; color: #fff; margin-bottom: 4px; }
        .step-desc { font-size: 12px; color: #888; }
        .btn-orange { display: block; text-align: center; text-decoration: none; background: #ff5500; color: #fff; font-weight: bold; font-size: 13px; padding: 12px 20px; border-radius: 12px; transition: background 0.2s; margin-top: 8px; }
        .btn-orange:hover { background: #e04b00; }
        .bookmarklet-btn { display: inline-block; cursor: grab; background: #262335; border: 1px dashed rgba(255,85,0,0.5); color: #ff7722; font-size: 12px; font-weight: 600; padding: 10px 16px; border-radius: 10px; text-decoration: none; margin-top: 8px; }
        .bookmarklet-btn:hover { background: #322e47; }
        .input-row { display: flex; gap: 8px; margin-top: 10px; }
        input[type="text"] { flex: 1; background: #0c0a10; border: 1px solid rgba(255,255,255,0.15); border-radius: 10px; padding: 10px 12px; color: #fff; font-size: 12px; font-family: monospace; outline: none; }
        input[type="text"]:focus { border-color: #ff5500; }
        button.submit-btn { background: #fff; color: #000; border: none; font-weight: bold; font-size: 12px; padding: 0 16px; border-radius: 10px; cursor: pointer; }
    </style>
</head>
<body>
    <div class="card">
        <div class="badge">🌐 Браузерный вход</div>
        <h1>Подключение SoundCloud</h1>
        <p>Войдите в ваш аккаунт SoundCloud через ваш привычный браузер. Все пароли и Google-аккаунт сохраняются в вашем браузере.</p>

        <div class="steps">
            <div class="step">
                <div class="step-num">ШАГ 1</div>
                <div class="step-title">Откройте soundcloud.com</div>
                <div class="step-desc">Убедитесь, что вы авторизованы в вашем профиле на сайте.</div>
                <a href="https://soundcloud.com/signin" target="_blank" class="btn-orange">Открыть SoundCloud в новой вкладке ↗</a>
            </div>

            <div class="step">
                <div class="step-num">ШАГ 2 (МГНОВЕННО)</div>
                <div class="step-title">Закладка быстрого подключения</div>
                <div class="step-desc">Перетащите кнопку ниже на панель закладок браузера, затем нажмите её, находясь на странице soundcloud.com:</div>
                <a class="bookmarklet-btn" href="javascript:(function(){const m=document.cookie.match(/oauth_token=([^;]+)/);if(!m){alert('Сначала войдите в SoundCloud на этой вкладке!');return;}fetch('http://127.0.0.1:49221/auth?token='+encodeURIComponent(m[1])).then(r=>r.text()).then(()=>{alert('✅ Успешно! Аккаунт подключен к SoundCloud Player.');window.location.reload();}).catch(()=>alert('Запустите десктопный плеер SoundCloud!'));})();">🔗 Подключить к SoundCloud Player</a>
            </div>

            <div class="step">
                <div class="step-num">ИЛИ ВРУЧНУЮ</div>
                <div class="step-title">Вставьте oauth_token из Cookies</div>
                <div class="input-row">
                    <input id="tok" type="text" placeholder="2-29... или OAuth ...">
                    <button class="submit-btn" onclick="save()">Войти</button>
                </div>
            </div>
        </div>
    </div>

    <script>
        function save() {
            const v = document.getElementById('tok').value.trim();
            if (!v) return;
            window.location.href = '/auth?token=' + encodeURIComponent(v);
        }
    </script>
</body>
</html>"#;

                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                        }
                    }
                });
            }
        }
        });
    });
}

fn parse_range_header(header: &str, file_size: u64) -> (u64, u64) {
    if file_size == 0 {
        return (0, 0);
    }
    let max_idx = file_size.saturating_sub(1);
    let clean = header.trim();
    if let Some(eq_idx) = clean.find('=') {
        let range_spec = &clean[eq_idx + 1..];
        let mut parts = range_spec.split('-');
        let start_str = parts.next().unwrap_or("").trim();
        let end_str = parts.next().unwrap_or("").trim();

        if start_str.is_empty() && !end_str.is_empty() {
            let suffix_len: u64 = end_str.parse().unwrap_or(0);
            let start = file_size.saturating_sub(suffix_len);
            return (start, max_idx);
        } else if !start_str.is_empty() {
            let start: u64 = start_str.parse().unwrap_or(0);
            let end: u64 = if end_str.is_empty() {
                max_idx
            } else {
                end_str.parse().unwrap_or(max_idx)
            };
            return (std::cmp::min(start, max_idx), std::cmp::min(end, max_idx));
        }
    }
    (0, max_idx)
}

fn urldecode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    result
}
