#![windows_subsystem = "windows"]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Fix WebKitGTK 2.42+ DMABUF hardware rendering black screen bug in AppImage & Wayland/X11
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
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
