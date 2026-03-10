//! Unix socket implementation for local IPC.

use std::io;
use std::path::Path;
use tokio::net::{UnixListener, UnixStream};

/// A local IPC stream (Unix socket on Unix platforms).
pub type LocalStream = UnixStream;

/// A local IPC listener (Unix socket listener on Unix platforms).
pub struct LocalListener {
    inner: UnixListener,
}

impl LocalListener {
    /// Bind to the given socket path.
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let inner = UnixListener::bind(path)?;
        Ok(Self { inner })
    }

    /// Accept a new connection.
    pub async fn accept(&self) -> io::Result<LocalStream> {
        let (stream, _addr) = self.inner.accept().await?;
        Ok(stream)
    }
}

/// Connect to a local IPC endpoint.
pub async fn connect(path: impl AsRef<Path>) -> io::Result<LocalStream> {
    UnixStream::connect(path).await
}

/// Check if a local IPC endpoint exists.
pub fn endpoint_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Remove a local IPC endpoint.
pub fn remove_endpoint(path: impl AsRef<Path>) -> io::Result<()> {
    std::fs::remove_file(path)
}
