//! Windows named pipe implementation for local IPC.

use std::io;
use std::path::Path;
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};

/// A local IPC stream (named pipe on Windows).
pub type LocalStream = NamedPipeClient;

/// A local IPC server stream (the connected server end of a named pipe).
pub type LocalServerStream = NamedPipeServer;

/// A local IPC listener (named pipe server on Windows).
pub struct LocalListener {
    pipe_name: String,
    next_server: NamedPipeServer,
}

fn normalize_pipe_name(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let pipe_name = path.as_os_str().to_string_lossy();

    if pipe_name.starts_with(r"\\.\pipe\") {
        pipe_name.into_owned()
    } else {
        crate::path_to_pipe_name(path)
    }
}

impl LocalListener {
    /// Bind to the given pipe name.
    pub fn bind(pipe_name: impl AsRef<Path>) -> io::Result<Self> {
        let pipe_name = normalize_pipe_name(pipe_name);
        let next_server = ServerOptions::new().create(&pipe_name)?;

        Ok(Self {
            pipe_name,
            next_server,
        })
    }

    /// Accept a new connection.
    pub async fn accept(&mut self) -> io::Result<LocalServerStream> {
        self.next_server.connect().await?;

        let connected = std::mem::replace(
            &mut self.next_server,
            ServerOptions::new().create(&self.pipe_name)?,
        );

        Ok(connected)
    }
}

/// Connect to a local IPC endpoint.
pub async fn connect(pipe_name: impl AsRef<Path>) -> io::Result<LocalStream> {
    let pipe_name = normalize_pipe_name(pipe_name);

    loop {
        match ClientOptions::new().open(&pipe_name) {
            Ok(client) => return Ok(client),
            Err(e) if e.raw_os_error() == Some(231) => {
                // Windows の named pipe が busy の時だけ短く待って再試行する。
                moire::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Check if a local IPC endpoint exists.
pub fn endpoint_exists(pipe_name: impl AsRef<Path>) -> bool {
    let pipe_name = normalize_pipe_name(pipe_name);

    match ClientOptions::new().open(&pipe_name) {
        Ok(_) => true,
        Err(e) => e.raw_os_error() == Some(231),
    }
}

/// Remove a local IPC endpoint.
pub fn remove_endpoint(_pipe_name: impl AsRef<Path>) -> io::Result<()> {
    Ok(())
}
