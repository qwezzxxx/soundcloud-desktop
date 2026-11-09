use serde_json::{json, Value};
use std::{io, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::watch,
    time::{sleep, sleep_until, timeout, Instant},
};

#[cfg(windows)]
type Pipe = tokio::net::windows::named_pipe::NamedPipeClient;
#[cfg(not(windows))]
type Pipe = tokio::net::UnixStream;

const RPC_TIMEOUT: Duration = Duration::from_secs(3);
const UPDATE_INTERVAL: Duration = Duration::from_millis(200);
const MAX_FRAME_SIZE: usize = 1024 * 1024;

#[derive(Clone, PartialEq)]
pub(super) struct Presence {
    pub client_id: String,
    pub activity: Value,
}

pub(super) async fn run(mut desired: watch::Receiver<Option<Presence>>) {
    let mut retry = Duration::from_secs(1);
    loop {
        let latest = desired.borrow_and_update().clone();
        let Some(presence) = latest.filter(|presence| !presence.activity.is_null()) else {
            if desired.changed().await.is_err() {
                return;
            }
            retry = Duration::from_secs(1);
            continue;
        };

        if let Some(pipe) = open_pipe().await {
            let started = Instant::now();
            let mut connection = Connection::new(pipe);
            let result = match connection.handshake(&presence.client_id).await {
                Ok(()) => connection.serve(&presence.client_id, &mut desired).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(()) => {
                    retry = Duration::from_secs(1);
                    continue;
                }
                Err(error) => {
                    crate::log_debug(format!("[Discord RPC] {error}"));
                    if error.kind() == io::ErrorKind::WouldBlock {
                        sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                }
            }
            if started.elapsed() > Duration::from_secs(10) {
                retry = Duration::from_secs(1);
            }
        }
        sleep(retry).await;
        retry = (retry * 2).min(Duration::from_secs(5));
    }
}

#[cfg(windows)]
async fn open_pipe() -> Option<Pipe> {
    (0..10).find_map(|index| {
        tokio::net::windows::named_pipe::ClientOptions::new()
            .open(format!(r"\\.\pipe\discord-ipc-{index}"))
            .ok()
    })
}

#[cfg(not(windows))]
async fn open_pipe() -> Option<Pipe> {
    use std::path::PathBuf;

    let mut prefixes = Vec::new();
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        prefixes.push(runtime.join("app/com.discordapp.Discord"));
        prefixes.push(runtime.join(".flatpak/org.equicord.equibop/xdg-run"));
        prefixes.push(runtime.join(".flatpak/com.discordapp.Discord/xdg-run"));
        prefixes.insert(0, runtime);
    }
    if let Some(tmp) = std::env::var_os("TMPDIR") {
        prefixes.push(PathBuf::from(tmp));
    }
    prefixes.push(PathBuf::from("/tmp"));

    for prefix in prefixes {
        for index in 0..10 {
            if let Ok(pipe) = Pipe::connect(prefix.join(format!("discord-ipc-{index}"))).await {
                return Some(pipe);
            }
        }
    }
    None
}

struct Connection<S> {
    stream: S,
    input: Vec<u8>,
    nonce: u64,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Connection<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            input: Vec::new(),
            nonce: 0,
        }
    }

    async fn write_frame(&mut self, opcode: u32, payload: &[u8]) -> io::Result<()> {
        let mut frame = Vec::with_capacity(8 + payload.len());
        frame.extend_from_slice(&opcode.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        self.stream.write_all(&frame).await
    }

    async fn read_frame(&mut self) -> io::Result<(u32, Vec<u8>)> {
        // Preserve partial reads when a newer playback state interrupts the idle wait.
        loop {
            if self.input.len() >= 8 {
                let opcode = u32::from_le_bytes(self.input[..4].try_into().unwrap());
                let length = u32::from_le_bytes(self.input[4..8].try_into().unwrap()) as usize;
                if length > MAX_FRAME_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Discord RPC frame too large",
                    ));
                }
                if self.input.len() >= 8 + length {
                    let payload = self.input[8..8 + length].to_vec();
                    self.input.drain(..8 + length);
                    return Ok((opcode, payload));
                }
            }
            self.input.reserve(4096);
            if self.stream.read_buf(&mut self.input).await? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Discord closed the connection",
                ));
            }
        }
    }

    async fn handle_frame(&mut self, opcode: u32, payload: Vec<u8>) -> io::Result<Option<Value>> {
        match opcode {
            1 => serde_json::from_slice(&payload)
                .map(Some)
                .map_err(io::Error::other),
            2 => {
                let code = serde_json::from_slice::<Value>(&payload)
                    .ok()
                    .and_then(|response| response["code"].as_i64());
                let kind = if code == Some(4002) {
                    io::ErrorKind::WouldBlock
                } else {
                    io::ErrorKind::ConnectionAborted
                };
                Err(io::Error::new(
                    kind,
                    format!("Discord closed the connection ({code:?})"),
                ))
            }
            3 => {
                timeout(RPC_TIMEOUT, self.write_frame(4, &payload)).await??;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn read_response(&mut self) -> io::Result<Value> {
        loop {
            let (opcode, payload) = self.read_frame().await?;
            if let Some(response) = self.handle_frame(opcode, payload).await? {
                return Ok(response);
            }
        }
    }

    async fn handshake(&mut self, client_id: &str) -> io::Result<()> {
        timeout(RPC_TIMEOUT, async {
            let hello = json!({ "v": 1, "client_id": client_id }).to_string();
            self.write_frame(0, hello.as_bytes()).await?;
            loop {
                let response = self.read_response().await?;
                match response["evt"].as_str() {
                    Some("READY") => return Ok(()),
                    Some("ERROR") => {
                        return Err(io::Error::other("Discord rejected the RPC handshake"));
                    }
                    _ => {}
                }
            }
        })
        .await?
    }

    async fn set_activity(&mut self, activity: &Value) -> io::Result<bool> {
        self.nonce += 1;
        let nonce = self.nonce.to_string();
        let request = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": nonce,
        })
        .to_string();
        timeout(RPC_TIMEOUT, async {
            self.write_frame(1, request.as_bytes()).await?;
            loop {
                let response = self.read_response().await?;
                if response["nonce"].as_str() != Some(nonce.as_str()) {
                    continue;
                }
                if response["evt"] == "ERROR" {
                    let code = response["data"]["code"].as_i64();
                    if code == Some(4000) || code == Some(4006) {
                        crate::log_debug(format!(
                            "[Discord RPC] Discord rejected the activity ({code:?})"
                        ));
                        return Ok(false);
                    }
                    return Err(io::Error::other(format!(
                        "Discord RPC update failed ({code:?})"
                    )));
                }
                if response["cmd"] == "SET_ACTIVITY" {
                    return Ok(true);
                }
            }
        })
        .await?
    }

    async fn serve(
        &mut self,
        client_id: &str,
        desired: &mut watch::Receiver<Option<Presence>>,
    ) -> io::Result<()> {
        let mut applied = Value::Null;
        let mut rejected = None;
        let mut next_update = Instant::now();
        loop {
            let latest = desired.borrow_and_update().clone();
            let Some(presence) = latest.filter(|presence| presence.client_id == client_id) else {
                if !applied.is_null() {
                    self.set_activity(&Value::Null).await?;
                }
                return Ok(());
            };
            let dirty =
                presence.activity != applied && rejected.as_ref() != Some(&presence.activity);
            tokio::select! {
                change = desired.changed() => {
                    if change.is_err() {
                        return Ok(());
                    }
                }
                frame = self.read_frame() => {
                    let (opcode, payload) = frame?;
                    self.handle_frame(opcode, payload).await?;
                }
                _ = sleep_until(next_update), if dirty => {
                    // Playback may have changed while waiting. Send only the most recent state.
                    if desired.has_changed().unwrap_or(true) {
                        continue;
                    }
                    next_update = Instant::now() + UPDATE_INTERVAL;
                    if self.set_activity(&presence.activity).await? {
                        if option_env!("ONYX_DEV_BUILD") == Some("1") {
                            let action = presence.activity["assets"]["small_image"]
                                .as_str()
                                .unwrap_or("clear");
                            crate::log_debug(format!("[Discord RPC] Activity confirmed: {action}"));
                        }
                        applied = presence.activity;
                        rejected = None;
                    } else {
                        rejected = Some(presence.activity);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sends_latest_state_after_ack_and_replays_it_after_disconnect() {
        let client_id = "123";
        let playing = json!({"assets": {"small_image": "pause"}});
        let paused = json!({"assets": {"small_image": "play"}});
        let (updates, mut desired) = watch::channel(Some(Presence {
            client_id: client_id.into(),
            activity: playing.clone(),
        }));

        let (stream, peer) = tokio::io::duplex(4096);
        let session =
            tokio::spawn(
                async move { Connection::new(stream).serve(client_id, &mut desired).await },
            );
        let mut discord = Connection::new(peer);
        let first = timeout(RPC_TIMEOUT, discord.read_response())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first["args"]["activity"], playing);

        for activity in [Value::Null, playing, paused.clone()] {
            updates.send_replace(Some(Presence {
                client_id: client_id.into(),
                activity,
            }));
        }
        // A reply with a different nonce must not release the in-flight update.
        discord
            .write_frame(1, br#"{"cmd":"SET_ACTIVITY","nonce":"other"}"#)
            .await
            .unwrap();
        assert!(timeout(UPDATE_INTERVAL * 2, discord.read_response())
            .await
            .is_err());
        let ack = json!({"cmd":"SET_ACTIVITY","nonce":first["nonce"]}).to_string();
        discord.write_frame(1, ack.as_bytes()).await.unwrap();
        let latest = timeout(RPC_TIMEOUT, discord.read_response())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest["args"]["activity"], paused);

        // Drop the pipe before Discord confirms the pause.
        drop(discord);
        assert!(session.await.unwrap().is_err());
        let (stream, peer) = tokio::io::duplex(4096);
        let mut desired = updates.subscribe();
        let session =
            tokio::spawn(
                async move { Connection::new(stream).serve(client_id, &mut desired).await },
            );
        let mut discord = Connection::new(peer);
        let replay = timeout(RPC_TIMEOUT, discord.read_response())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replay["args"]["activity"], paused);
        let ack = json!({"cmd":"SET_ACTIVITY","nonce":replay["nonce"]}).to_string();
        discord.write_frame(1, ack.as_bytes()).await.unwrap();

        updates.send_replace(None);
        let clear = timeout(RPC_TIMEOUT, discord.read_response())
            .await
            .unwrap()
            .unwrap();
        assert!(clear["args"]["activity"].is_null());
        let ack = json!({"cmd":"SET_ACTIVITY","nonce":clear["nonce"]}).to_string();
        discord.write_frame(1, ack.as_bytes()).await.unwrap();
        session.await.unwrap().unwrap();
    }
}
