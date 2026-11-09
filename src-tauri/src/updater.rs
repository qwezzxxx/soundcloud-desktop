use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

pub fn start(app: &AppHandle) -> tauri::Result<()> {
    // Keep development artifacts available for testing across restarts.
    if option_env!("ONYX_DEV_BUILD") == Some("1") {
        return Ok(());
    }

    app.plugin(tauri_plugin_updater::Builder::new().build())?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = update(&app).await {
            crate::log_debug(format!("[Updater] {error}"));
        }
    });
    Ok(())
}

async fn update(app: &AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exe = std::env::current_exe()?;
    let previous = exe.with_added_extension("old-onyx-update");
    let update = app
        .updater_builder()
        .configure_client(|client| {
            client
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(90))
        })
        .build()?
        .check()
        .await?;
    if update.is_none() && !previous.exists() {
        return Ok(());
    }

    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(exe.with_added_extension("onyx-update-lock.tmp"))?;
    lock.try_lock()?;

    // A running copy of the previous version still holds its renamed EXE open.
    // In that case Windows refuses deletion, so leave the new EXE in place.
    match fs::remove_file(&previous) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    if let Some(update) = update {
        let bytes = update.download(|_, _| {}, || {}).await?;
        replace_executable(&exe, &previous, &bytes)?;
        crate::log_debug(format!(
            "[Updater] ONYX {} will run on the next launch",
            update.version
        ));
    }
    Ok(())
}

fn replace_executable(exe: &Path, previous: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ONYX executable has no directory",
        )
    })?;
    let mut staged = tempfile::Builder::new()
        .suffix(".tmp")
        .tempfile_in(directory)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    let staged = staged.into_temp_path();

    // Windows permits moving a running executable aside within the same volume.
    fs::rename(exe, previous)?;
    if let Err(error) = staged.persist_noclobber(exe) {
        if !exe.exists() {
            fs::rename(previous, exe)?;
        }
        return Err(error.error);
    }
    Ok(())
}
