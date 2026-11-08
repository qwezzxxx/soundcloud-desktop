use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use tokio::net::{TcpListener, TcpStream};
use crate::soundcloud::SoundCloudClient;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
const AUTH_ADDRESS: &str = "127.0.0.1:49221";
const LOGIN_TTL: Duration = Duration::from_secs(600);
static BROWSER_LOGIN: Mutex<Option<(String, Instant)>> = Mutex::new(None);
static BROWSER_AUTH_REQUEST: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn begin_browser_login() -> Result<&'static str, String> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|error| error.to_string())?;
    let mut nonce = String::with_capacity(random.len() * 2);
    for byte in random {
        let _ = write!(nonce, "{byte:02x}");
    }
    let mut login = BROWSER_LOGIN.lock().map_err(|error| error.to_string())?;
    *login = Some((nonce, Instant::now()));
    Ok("http://127.0.0.1:49221/login")
}

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_default()
    })
}

pub fn start_auth_server(sc: Arc<SoundCloudClient>, app: AppHandle) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Auth server runtime init error: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            let listener = match TcpListener::bind(AUTH_ADDRESS).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Auth server bind failed: {}", e);
                    return;
                }
            };

            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let sc_ref = sc.clone();
                    let app = app.clone();
                    tokio::spawn(async move {
                        let mut buffer = [0u8; 16384];
                    if let Ok(Some((bytes_read, headers_end))) =
                        tokio::time::timeout(Duration::from_secs(5), read_headers(&mut stream, &mut buffer)).await
                    {
                        let request_str = String::from_utf8_lossy(&buffer[..headers_end]);
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

                        let host = request_header(&request_str, "Host");
                        if host != Some(AUTH_ADDRESS) {
                            return;
                        }

                        match (method, path) {
                            ("GET", "/login") => {
                                let nonce = BROWSER_LOGIN.lock().ok().and_then(|login| {
                                    login.as_ref()
                                        .filter(|(_, started)| started.elapsed() < LOGIN_TTL)
                                        .map(|(nonce, _)| nonce.clone())
                                });
                                if let Some(nonce) = nonce {
                                    write_login_page(&mut stream, "200 OK", &LOGIN_STEPS.replace("__NONCE__", &nonce)).await;
                                } else {
                                    write_login_notice(&mut stream, "403 Forbidden", "Вход в SoundCloud", "Откройте страницу подключения из ONYX.").await;
                                }
                            }
                            ("POST", "/auth") => {
                                let length = request_header(&request_str, "Content-Length")
                                    .and_then(|length| length.parse::<usize>().ok())
                                    .filter(|&length| length <= 4096);
                                let Some(end) = length.and_then(|length| headers_end.checked_add(length))
                                    .filter(|&end| end > headers_end && end <= buffer.len())
                                else {
                                    write_login_page(&mut stream, "400 Bad Request", "Неверный запрос подключения.").await;
                                    return;
                                };
                                if bytes_read < end
                                    && !matches!(
                                        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buffer[bytes_read..end])).await,
                                        Ok(Ok(_))
                                    )
                                {
                                    return;
                                }
                                let form = String::from_utf8_lossy(&buffer[headers_end..end]);
                                let field = |key| {
                                    form.split('&')
                                        .filter_map(|field| field.split_once('='))
                                        .find_map(|(name, value)| (name == key).then(|| urldecode(value)))
                                };
                                let Some(token) = field("token").filter(|value| value.len() >= 10) else {
                                    write_login_page(&mut stream, "400 Bad Request", "Войдите в SoundCloud и повторите подключение.").await;
                                    return;
                                };
                                let _request = BROWSER_AUTH_REQUEST.lock().await;
                                if !field("nonce").is_some_and(|nonce| consume_browser_login(&nonce)) {
                                    write_login_notice(&mut stream, "403 Forbidden", "Срок подключения истёк", "Начните вход заново из ONYX.").await;
                                    return;
                                }

                                match sc_ref.set_oauth_token(&token).await {
                                    Ok(data) => {
                                        let _ = app.emit("sc_auth_success", data);
                                        write_login_notice(&mut stream, "200 OK", "Аккаунт подключён", "Закройте вкладку и вернитесь в ONYX.").await;
                                    }
                                    Err(_) => {
                                        let message = "SoundCloud отклонил подключение. Начните вход заново из ONYX.";
                                        let _ = app.emit("sc_auth_error", message);
                                        write_login_notice(&mut stream, "401 Unauthorized", "Подключение прервано", message).await;
                                    }
                                }
                            }
                            _ => write_login_page(&mut stream, "404 Not Found", "Страница не найдена.").await,
                        }
                    }
                });
            }
        }
        });
    });
}

fn consume_browser_login(nonce: &str) -> bool {
    let Ok(mut login) = BROWSER_LOGIN.lock() else {
        return false;
    };
    if login.as_ref().is_some_and(|(expected, started)| expected == nonce && started.elapsed() < LOGIN_TTL) {
        *login = None;
        return true;
    }
    false
}

async fn read_headers(stream: &mut TcpStream, buffer: &mut [u8]) -> Option<(usize, usize)> {
    let mut bytes_read = 0;
    loop {
        let count = stream.read(&mut buffer[bytes_read..]).await.ok()?;
        if count == 0 {
            return None;
        }
        bytes_read += count;
        if let Some(end) = buffer[..bytes_read].windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            return Some((bytes_read, end + 4));
        }
    }
}

fn request_header<'a>(headers: &'a str, key: &str) -> Option<&'a str> {
    headers.lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| name.eq_ignore_ascii_case(key).then(|| value.trim()))
}

async fn write_login_notice(stream: &mut TcpStream, status: &str, title: &str, message: &str) {
    let body = format!(
        r#"<main class="result"><div class="brand">ONYX</div><h1>{title}</h1><p>{message}</p></main>"#
    );
    write_login_page(stream, status, &body).await;
}

async fn write_login_page(stream: &mut TcpStream, status: &str, body: &str) {
    let page = format!("{LOGIN_HEAD}{body}</body></html>");
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

const LOGIN_HEAD: &str = r#"<!doctype html>
<html lang="ru">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <title>SoundCloud · ONYX</title>
    <style>
        :root { color-scheme: dark; --accent: #ff8554; --muted: #b9b6c3; }
        * { box-sizing: border-box; }
        body { margin: 0; min-height: 100vh; display: grid; place-items: center; padding: 24px; background: #0d0d12; color: #f9f8fb; font: 15px/1.5 system-ui, sans-serif; }
        main { width: min(880px, 100%); overflow: hidden; border: 1px solid #36333f; border-radius: 24px; background: #16151d; box-shadow: 0 24px 80px #0007; }
        .connect { display: grid; grid-template-columns: 1fr 1.2fr; }
        .intro { display: flex; flex-direction: column; justify-content: center; gap: 16px; padding: 36px; background: radial-gradient(ellipse at top left, #542c29 0%, #231b26 58%, #1c1922 100%); }
        .brand { color: #ffc1a8; font-size: 12px; font-weight: 750; letter-spacing: .2em; }
        h1 { font-size: clamp(30px, 5vw, 44px); line-height: 1.05; letter-spacing: -.045em; margin: 0; }
        p { margin: 0; color: var(--muted); }
        .wave { width: 104px; height: 40px; color: var(--accent); }
        ol { list-style: none; counter-reset: steps; margin: 0; padding: 30px; display: grid; gap: 14px; }
        li { counter-increment: steps; display: grid; grid-template-columns: 34px minmax(0, 1fr); align-items: start; gap: 12px; }
        li::before { content: counter(steps, decimal-leading-zero); display: grid; place-items: center; width: 34px; height: 34px; border: 1px solid #583a36; border-radius: 10px; background: #2d2126; color: #ffb495; font-size: 12px; font-weight: 750; }
        h2 { font-size: 15px; margin: 4px 0 5px; }
        li p { font-size: 13px; }
        a:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; }
        .button { display: inline-flex; align-items: center; gap: 8px; margin: 10px 0 0; padding: 8px 12px; border-radius: 9px; color: #19100b; background: var(--accent); text-decoration: none; font-weight: 750; font-size: 13px; }
        .button:hover { background: #ffa27e; }
        .bookmark { color: #ffc1a8; background: #241d24; border: 1px dashed #a8604c; cursor: grab; }
        .bookmark:hover { background: #3a292f; }
        kbd { color: #f2e9f0; white-space: nowrap; }
        .result { width: min(500px, 100%); padding: 32px; display: grid; gap: 14px; }
        .result h1 { font-size: 30px; }
        @media (max-width: 680px) {
            body { padding: 14px; }
            .connect { grid-template-columns: 1fr; }
            .intro { padding: 24px; gap: 12px; }
            ol { padding: 22px; }
        }
    </style>
</head>
<body>"#;

const LOGIN_STEPS: &str = r#"<main class="connect">
    <header class="intro">
        <div class="brand">ONYX</div>
        <svg class="wave" aria-hidden="true" viewBox="0 0 104 40" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round">
            <path d="M4 17v6m12-12v18m12-24v30m12-21v12m12-23v34m12-27v20m12-14v8m12-12v16m12-12v8"/>
        </svg>
        <h1>Подключите<br>SoundCloud</h1>
        <p>Войдите в SoundCloud и синхронизируйте лайки в ONYX.</p>
    </header>
    <ol>
        <li><div>
            <h2>Добавьте закладку</h2>
            <p>Перетащите кнопку на панель закладок. В Chrome и Edge панель открывается через <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>B</kbd>.</p>
            <a class="button bookmark" onclick="event.preventDefault(); alert('Перетащите эту кнопку на панель закладок.');" href="javascript:(function(){if(location.origin!=='https://soundcloud.com'){alert('Откройте soundcloud.com и войдите в аккаунт.');return;}const match=document.cookie.match(/(?:^|;\s*)oauth_token=([^;]+)/);if(!match){alert('Сессия SoundCloud недоступна. Войдите и попробуйте ещё раз.');return;}const form=document.createElement('form');form.method='POST';form.action='http://127.0.0.1:49221/auth';for(const [name,value] of Object.entries({nonce:'__NONCE__',token:decodeURIComponent(match[1])})){const input=document.createElement('input');input.type='hidden';input.name=name;input.value=value;form.append(input);}document.body.append(form);form.submit();})();">Подключить к ONYX</a>
        </div></li>
        <li><div>
            <h2>Войдите в SoundCloud</h2>
            <p>Выберите способ входа на сайте SoundCloud.</p>
            <a class="button" href="https://soundcloud.com/signin" target="_blank" rel="noopener noreferrer">Открыть SoundCloud <span aria-hidden="true">↗</span></a>
        </div></li>
        <li><div>
            <h2>Подключите аккаунт</h2>
            <p>На вкладке SoundCloud нажмите закладку <strong>«Подключить к ONYX»</strong>. После подтверждения вернитесь в плеер.</p>
        </div></li>
    </ol>
</main>"#;

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
