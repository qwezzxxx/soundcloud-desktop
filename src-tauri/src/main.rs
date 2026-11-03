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
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };
        let msg = format!("{}: location {:?}", payload, info.location());
        let _ = std::fs::write("panic.log", &msg);
    }));
    app_lib::run();
}
