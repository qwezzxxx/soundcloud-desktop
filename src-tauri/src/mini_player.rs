use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Rect, Runtime, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};

const LABEL: &str = "mini-player";
const WIDTH: f64 = 404.0;
const HEIGHT: f64 = 142.0;

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("mini_player.html".into()))
        .title("ONYX — Мини-плеер")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focused(false)
        .visible(false);
    // WebView2 requires the same browser options for windows sharing a profile.
    if let Some(args) = app.config().app.windows.iter()
        .find(|window| window.label == "main")
        .and_then(|window| window.additional_browser_args.as_deref())
    {
        builder = builder.additional_browser_args(args);
    }
    let window = builder.build()?;
    let popup = window.clone();
    window.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(false) => {
                let _ = popup.hide();
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = popup.hide();
            }
            _ => {}
        }
    });
    Ok(())
}

pub fn show_at_tray<R: Runtime>(app: &AppHandle<R>, pointer: PhysicalPosition<f64>, icon: Rect) -> bool {
    let Some(window) = app.get_webview_window(LABEL) else {
        return false;
    };
    if let Ok(Some(monitor)) = app.monitor_from_point(pointer.x, pointer.y) {
        let scale = monitor.scale_factor();
        let work = monitor.work_area();
        let left = work.position.x;
        let top = work.position.y;
        let right = left + work.size.width as i32;
        let bottom = top + work.size.height as i32;
        let size = PhysicalSize::new((WIDTH * scale).round() as u32, (HEIGHT * scale).round() as u32);
        let gap = (8.0 * scale).round() as i32;
        let icon_pos: PhysicalPosition<i32> = icon.position.to_physical(scale);
        let icon_size: PhysicalSize<i32> = icon.size.to_physical(scale);
        let width = size.width as i32;
        let height = size.height as i32;
        let x = (icon_pos.x + icon_size.width - width).clamp(left, (right - width).max(left));
        let y = (icon_pos.y.min(bottom) - gap - height).clamp(top, (bottom - height).max(top));
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(size);
    }
    window.show().and_then(|()| window.set_focus()).is_ok()
}
