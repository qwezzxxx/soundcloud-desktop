use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::soundcloud::SoundCloudClient;

static HTTP_CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
const AUTH_ADDRESS: &str = "127.0.0.1:49221";
static BROWSER_AUTH_REQUEST: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) fn random_ticket() -> Result<String, String> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|error| error.to_string())?;
    let mut nonce = String::with_capacity(random.len() * 2);
    for byte in random {
        let _ = write!(nonce, "{byte:02x}");
    }
    Ok(nonce)
}

pub fn begin_browser_login() -> Result<String, String> {
    Ok(format!("http://{AUTH_ADDRESS}/login"))
}

const STREAM_TTL: Duration = Duration::from_secs(3600);
const MAX_STREAM_TICKETS: usize = 2048;
static STREAM_TICKETS: OnceLock<Mutex<HashMap<String, (tauri::Url, Instant)>>> = OnceLock::new();

fn is_youtube_media_url(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.port_or_known_default() == Some(443)
        && url.username().is_empty() && url.password().is_none()
        && url.host_str().is_some_and(|host| host == "googlevideo.com" || host.ends_with(".googlevideo.com"))
}

pub fn register_youtube_stream(raw: &str) -> Option<String> {
    let url = tauri::Url::parse(raw).ok()?;
    if !is_youtube_media_url(&url) { return None; }
    let key = random_ticket().ok()?;
    let mut tickets = STREAM_TICKETS.get_or_init(Mutex::default).lock().ok()?;
    tickets.retain(|_, (_, created)| created.elapsed() < STREAM_TTL);
    if tickets.len() >= MAX_STREAM_TICKETS {
        let oldest = tickets.iter().min_by_key(|(_, (_, created))| *created)?.0.clone();
        tickets.remove(&oldest);
    }
    tickets.insert(key.clone(), (url, Instant::now()));
    Some(format!("http://{AUTH_ADDRESS}/stream?key={key}"))
}

pub fn is_registered_stream_url(url: &tauri::Url) -> bool {
    url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port() == Some(49221)
        && url.path() == "/stream" && url.query().and_then(|query| query.strip_prefix("key=")).is_some_and(|key| stream_url_for_ticket(key).is_some())
}

fn stream_url_for_ticket(key: &str) -> Option<tauri::Url> {
    let tickets = STREAM_TICKETS.get()?.lock().ok()?;
    let (url, created) = tickets.get(key)?;
    (created.elapsed() < STREAM_TTL).then(|| url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_accepts_only_registered_youtube_media() {
        for rejected in [
            "http://127.0.0.1:8080/admin",
            "https://googlevideo.com.example.org/audio",
            "http://rr1.googlevideo.com/audio",
            "https://rr1.googlevideo.com:8080/audio",
            "https://name@rr1.googlevideo.com/audio",
        ] {
            assert!(register_youtube_stream(rejected).is_none(), "{rejected}");
        }
        let ticket = register_youtube_stream("https://rr1.googlevideo.com/audio").unwrap();
        let url = tauri::Url::parse(&ticket).unwrap();
        assert!(is_registered_stream_url(&url));
        assert!(!ticket.contains("googlevideo.com"));
        for rejected in [
            "http://127.0.0.1:49221/stream?key=guessed",
            "http://127.0.0.1:49221/stream?url=http://127.0.0.1/",
        ] {
            assert!(!is_registered_stream_url(&tauri::Url::parse(rejected).unwrap()));
        }
    }
}

fn get_http_client() -> Option<&'static reqwest::Client> {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .timeout(Duration::from_secs(45))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 || !is_youtube_media_url(attempt.url()) {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .ok()
    }).as_ref()
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

            let connections = Arc::new(tokio::sync::Semaphore::new(64));
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let Ok(permit) = connections.clone().try_acquire_owned() else { continue; };
                    let sc_ref = sc.clone();
                    let app = app.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
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

                        let host = request_header(&request_str, "Host");
                        if !matches!(host, Some("127.0.0.1:49221" | "localhost:49221" | "127.0.0.1" | "localhost")) {
                            write_empty(&mut stream, "403 Forbidden").await;
                            return;
                        }

                        match (method, path) {
                            ("GET" | "HEAD" | "OPTIONS", path) if path.starts_with("/stream?key=") => {
                                serve_stream(&mut stream, method, path, &request_str).await;
                            }
                            ("GET", path) if path == "/" || path == "/login" || path.starts_with("/login") => {
                                write_login_page(&mut stream, "200 OK", LOGIN_STEPS).await;
                            }
                            ("GET", path) if path.starts_with("/auth") => {
                                let token = if let Some(idx) = path.find("token=") {
                                    let raw = &path[idx + 6..];
                                    let end = raw.find('&').unwrap_or(raw.len());
                                    urldecode(&raw[..end])
                                } else {
                                    String::new()
                                };
                                let trimmed = token.trim();
                                if trimmed.len() >= 10 {
                                    let _request = BROWSER_AUTH_REQUEST.lock().await;
                                    match sc_ref.set_oauth_token(trimmed).await {
                                        Ok(data) => {
                                            let _ = app.emit("sc_auth_success", data);
                                            write_login_notice(&mut stream, "200 OK", "Аккаунт подключён", "Закройте вкладку и вернитесь в ONYX.").await;
                                        }
                                        Err(_) => {
                                            let message = "SoundCloud отклонил токен. Проверьте правильность и попробуйте ещё раз.";
                                            let _ = app.emit("sc_auth_error", message);
                                            write_login_notice(&mut stream, "401 Unauthorized", "Подключение прервано", message).await;
                                        }
                                    }
                                } else {
                                    write_login_page(&mut stream, "400 Bad Request", "Не указан токен авторизации.").await;
                                }
                            }
                            ("POST", "/auth") => {
                                let length = request_header(&request_str, "Content-Length")
                                    .and_then(|length| length.parse::<usize>().ok())
                                    .filter(|&length| length <= 8192);
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
                                let Some(token) = field("token").filter(|value| value.trim().len() >= 10) else {
                                    write_login_page(&mut stream, "400 Bad Request", "Войдите в SoundCloud и повторите подключение.").await;
                                    return;
                                };
                                let _request = BROWSER_AUTH_REQUEST.lock().await;
                                match sc_ref.set_oauth_token(token.trim()).await {
                                    Ok(data) => {
                                        let _ = app.emit("sc_auth_success", data);
                                        write_login_notice(&mut stream, "200 OK", "Аккаунт подключён", "Закройте вкладку и вернитесь в ONYX.").await;
                                    }
                                    Err(_) => {
                                        let message = "SoundCloud отклонил токен. Проверьте правильность и попробуйте ещё раз.";
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

async fn write_empty(stream: &mut TcpStream, status: &str) {
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let _ = stream.write_all(response.as_bytes()).await;
}

async fn serve_stream(stream: &mut TcpStream, method: &str, path: &str, headers: &str) {
    let key = &path["/stream?key=".len()..];
    let Some(url) = stream_url_for_ticket(key) else {
        write_empty(stream, "404 Not Found").await;
        return;
    };
    let origin = request_header(headers, "Origin").filter(|origin| {
        matches!(*origin, "http://tauri.localhost" | "https://tauri.localhost" | "tauri://localhost")
    });
    let cors = origin.map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n")).unwrap_or_default();
    if method == "OPTIONS" {
        if origin.is_none() {
            write_empty(stream, "403 Forbidden").await;
            return;
        }
        let response = format!("HTTP/1.1 204 No Content\r\n{cors}Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range\r\nConnection: close\r\n\r\n");
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    let Some(client) = get_http_client() else {
        write_empty(stream, "502 Bad Gateway").await;
        return;
    };
    let mut request = if method == "HEAD" { client.head(url) } else { client.get(url) };
    request = request.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36");
    if let Some(range) = request_header(headers, "Range") {
        request = request.header(reqwest::header::RANGE, range);
    }
    let Ok(mut response) = request.send().await else {
        write_empty(stream, "502 Bad Gateway").await;
        return;
    };
    let status = match response.status().as_u16() {
        200 => "200 OK",
        206 => "206 Partial Content",
        416 => "416 Range Not Satisfiable",
        _ => {
            write_empty(stream, "502 Bad Gateway").await;
            return;
        }
    };
    let media_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next()).filter(|value| value.starts_with("audio/") || *value == "application/octet-stream");
    if media_type.is_none() && response.status().is_success() {
        write_empty(stream, "502 Bad Gateway").await;
        return;
    }
    let media_type = media_type.unwrap_or("application/octet-stream");
    let mut head = format!("HTTP/1.1 {status}\r\n{cors}Content-Type: {media_type}\r\nX-Content-Type-Options: nosniff\r\nAccept-Ranges: bytes\r\nConnection: close\r\n");
    for name in [reqwest::header::CONTENT_LENGTH, reqwest::header::CONTENT_RANGE] {
        if let Some(value) = response.headers().get(&name).and_then(|value| value.to_str().ok()) {
            let _ = write!(head, "{name}: {value}\r\n");
        }
    }
    head.push_str("\r\n");
    if stream.write_all(head.as_bytes()).await.is_err() || method == "HEAD" { return; }
    while let Ok(Some(chunk)) = response.chunk().await {
        if stream.write_all(&chunk).await.is_err() { break; }
    }
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
        .token-row { display: flex; gap: 8px; margin-top: 10px; }
        .token-input { flex: 1; background: #0d0d12; border: 1px solid #36333f; border-radius: 9px; padding: 8px 12px; color: #fff; font-size: 13px; font-family: monospace; outline: none; }
        .token-input:focus { border-color: var(--accent); }
        .hint { font-size: 11px; color: #888; margin-top: 6px; }
        .hint code { color: #ffb495; background: #241d24; padding: 1px 4px; border-radius: 4px; }
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
            <h2>Войдите в SoundCloud</h2>
            <p>Откройте сайт soundcloud.com и авторизуйтесь в вашем профиле.</p>
            <a class="button" href="https://soundcloud.com/signin" target="_blank" rel="noopener noreferrer">Открыть SoundCloud <span aria-hidden="true">↗</span></a>
        </div></li>
        <li><div>
            <h2>Быстрое подключение через закладку</h2>
            <p>Перетащите кнопку на панель закладок браузера (<kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>B</kbd>). Затем перейдите на вкладку SoundCloud и нажмите её:</p>
            <a class="button bookmark" onclick="event.preventDefault(); alert('Перетащите эту кнопку на панель закладок браузера, затем откройте soundcloud.com и нажмите её на вкладке.');" href="javascript:(function(){if(location.origin!=='https://soundcloud.com'){alert('Откройте вкладку soundcloud.com и войдите в аккаунт!');return;}var t=null;var m=document.cookie.match(/(?:^|;\s*)oauth_token=([^;]+)/);if(m&&m[1]){t=decodeURIComponent(m[1]);}if(!t){try{for(var i=0;i<localStorage.length;i++){var k=localStorage.key(i);if(k==='oauth_token'||k==='sc_oauth_token'){var v=localStorage.getItem(k);if(v&&v.length>10){t=v;break;}}}}catch(e){}}if(!t){alert('Сессия SoundCloud не найдена. Убедитесь, что вы авторизованы на soundcloud.com!');return;}window.location.href='http://127.0.0.1:49221/auth?token='+encodeURIComponent(t);})();">🔗 Подключить к ONYX</a>
        </div></li>
        <li><div>
            <h2>Или вставьте токен прямо здесь</h2>
            <p>Скопируйте oauth_token из cookies или запросов и нажмите «Войти»:</p>
            <div class="token-row">
                <input id="token-input" class="token-input" type="text" placeholder="2-29... или OAuth ...">
                <button class="button" style="margin:0; cursor:pointer;" onclick="var val=document.getElementById('token-input').value.trim();if(!val){alert('Вставьте токен!');return;}window.location.href='/auth?token='+encodeURIComponent(val);">Войти</button>
            </div>
            <p class="hint">В браузере: нажмите <code>F12</code> на soundcloud.com → вкладка <code>Application</code> (Приложение) → <code>Cookies</code> → скопируйте <code>oauth_token</code>.</p>
        </div></li>
    </ol>
</main>"#;

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
