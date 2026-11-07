#![windows_subsystem = "windows"]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fix WebKitGTK 2.42+ DMABUF hardware rendering black screen bug in AppImage & Wayland/X11
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    #[cfg(target_os = "windows")]
    {
        // Allow background audio playback and timers when window is minimized or hidden in tray
        let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGS").unwrap_or_default();
        let bg_flags = "--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows --autoplay-policy=no-user-gesture-required";
        let new_args = if existing.is_empty() {
            bg_flags.to_string()
        } else {
            format!("{} {}", existing, bg_flags)
        };
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGS", new_args);
    }

    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };
        let bt = std::backtrace::Backtrace::force_capture();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let msg = format!("[{}] PANIC: {} | location {:?}\n{}\n", ts, payload, info.location(), bt);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("panic.log") {
            let _ = f.write_all(msg.as_bytes());
            let _ = f.flush();
        }
    }));
    app_lib::run();
}
