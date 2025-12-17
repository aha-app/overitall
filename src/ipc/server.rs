use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::task::{Context, Poll};

use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};

use super::protocol::{IpcRequest, IpcResponse};

/// Unique identifier for a client connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Buffered client connection state
struct ClientConnection {
    stream: UnixStream,
    buffer: String,
}

impl ClientConnection {
    fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            buffer: String::new(),
        }
    }
}

/// IPC server for handling CLI client connections via Unix socket
pub struct IpcServer {
    listener: UnixListener,
    connections: HashMap<ConnectionId, ClientConnection>,
    socket_path: PathBuf,
    next_conn_id: u64,
}

impl IpcServer {
    /// Create a new IPC server bound to the given socket path.
    /// Removes any stale socket file that may exist.
    pub fn new(socket_path: impl AsRef<Path>) -> io::Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();

        // Remove stale socket if it exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        let listener = UnixListener::bind(&socket_path)?;

        Ok(Self {
            listener,
            connections: HashMap::new(),
            socket_path,
            next_conn_id: 0,
        })
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Accept any pending new connections (non-blocking).
    /// Returns the number of new connections accepted.
    pub fn accept_pending(&mut self) -> io::Result<usize> {
        let mut accepted = 0;

        // Create a noop waker for polling
        let waker = futures_task_noop_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            match self.listener.poll_accept(&mut cx) {
                Poll::Ready(Ok((stream, _addr))) => {
                    let conn_id = ConnectionId::new(self.next_conn_id);
                    self.next_conn_id += 1;
                    self.connections.insert(conn_id, ClientConnection::new(stream));
                    accepted += 1;
                }
                Poll::Ready(Err(e)) => return Err(e),
                Poll::Pending => break,
            }
        }

        Ok(accepted)
    }

    /// Poll for incoming commands from all connected clients (non-blocking).
    /// Returns requests paired with their connection IDs.
    /// Disconnected clients are automatically removed.
    pub fn poll_commands(&mut self) -> io::Result<Vec<(ConnectionId, IpcRequest)>> {
        let mut requests = Vec::new();
        let mut disconnected = Vec::new();

        for (&conn_id, client) in &mut self.connections {
            // Try to read available data
            let mut buf = [0u8; 4096];
            loop {
                match client.stream.try_read(&mut buf) {
                    Ok(0) => {
                        // Connection closed
                        disconnected.push(conn_id);
                        break;
                    }
                    Ok(n) => {
                        // Append to buffer
                        if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                            client.buffer.push_str(s);
                        } else {
                            // Invalid UTF-8, disconnect client
                            disconnected.push(conn_id);
                            break;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        disconnected.push(conn_id);
                        break;
                    }
                }
            }

            // Parse complete JSON lines from buffer
            while let Some(newline_pos) = client.buffer.find('\n') {
                let line = client.buffer[..newline_pos].to_string();
                client.buffer = client.buffer[newline_pos + 1..].to_string();

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<IpcRequest>(&line) {
                    Ok(request) => {
                        requests.push((conn_id, request));
                    }
                    Err(_) => {
                        // Malformed JSON - we could send an error response
                        // For now, just ignore the malformed line
                    }
                }
            }
        }

        // Remove disconnected clients
        for conn_id in disconnected {
            self.connections.remove(&conn_id);
        }

        Ok(requests)
    }

    /// Send a response to a specific client connection.
    /// Returns an error if the connection no longer exists or write fails.
    pub async fn send_response(
        &mut self,
        conn_id: ConnectionId,
        response: IpcResponse,
    ) -> io::Result<()> {
        let client = self
            .connections
            .get_mut(&conn_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "connection not found"))?;

        let mut json = serde_json::to_string(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        json.push('\n');

        client.stream.write_all(json.as_bytes()).await?;
        client.stream.flush().await?;

        Ok(())
    }

    /// Close a specific client connection
    pub fn close_connection(&mut self, conn_id: ConnectionId) {
        self.connections.remove(&conn_id);
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Check if a connection exists
    pub fn has_connection(&self, conn_id: ConnectionId) -> bool {
        self.connections.contains_key(&conn_id)
    }

    /// Clean up the socket file
    pub fn cleanup(&self) -> io::Result<()> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Best-effort cleanup of socket file
        let _ = self.cleanup();
    }
}

/// Create a no-op waker for polling
fn futures_task_noop_waker() -> std::task::Waker {
    use std::task::{RawWaker, RawWakerVTable, Waker};

    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE), // clone
        |_| {},                                        // wake
        |_| {},                                        // wake_by_ref
        |_| {},                                        // drop
    );

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    fn temp_socket_path() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sock");
        (dir, path)
    }

    #[tokio::test]
    async fn server_binds_to_socket() {
        let (_dir, path) = temp_socket_path();
        let server = IpcServer::new(&path).unwrap();
        assert!(path.exists());
        assert_eq!(server.socket_path(), path);
    }

    #[tokio::test]
    async fn server_removes_stale_socket() {
        let (_dir, path) = temp_socket_path();

        // Create a stale socket file
        std::fs::write(&path, "stale").unwrap();
        assert!(path.exists());

        // Server should remove it and bind successfully
        let _server = IpcServer::new(&path).unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn server_cleanup_removes_socket() {
        let (_dir, path) = temp_socket_path();
        let server = IpcServer::new(&path).unwrap();
        assert!(path.exists());

        server.cleanup().unwrap();
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn server_accepts_connection() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        // Connect a client
        let _client = UnixStream::connect(&path).await.unwrap();

        // Give the connection time to be ready
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Accept pending connections
        let accepted = server.accept_pending().unwrap();
        assert_eq!(accepted, 1);
        assert_eq!(server.connection_count(), 1);
    }

    #[tokio::test]
    async fn server_receives_request() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        // Connect and send a request
        let mut client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Send a JSON request
        let request = IpcRequest::new("ping");
        let mut json = serde_json::to_string(&request).unwrap();
        json.push('\n');
        client.write_all(json.as_bytes()).await.unwrap();
        client.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Poll for commands
        let requests = server.poll_commands().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1.command, "ping");
    }

    #[tokio::test]
    async fn server_sends_response() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        // Connect client
        let client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Get the connection ID
        let conn_id = *server.connections.keys().next().unwrap();

        // Send a response
        let response = IpcResponse::ok(json!({"status": "ok"}));
        server.send_response(conn_id, response).await.unwrap();

        // Read the response on the client
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        let parsed: IpcResponse = serde_json::from_str(&line).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.result, Some(json!({"status": "ok"})));
    }

    #[tokio::test]
    async fn server_handles_client_disconnect() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        // Connect and disconnect client
        let client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();
        assert_eq!(server.connection_count(), 1);

        // Drop the client to disconnect
        drop(client);
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Poll should detect the disconnect
        let _ = server.poll_commands().unwrap();
        assert_eq!(server.connection_count(), 0);
    }

    #[tokio::test]
    async fn server_handles_multiple_requests_in_buffer() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Send multiple requests at once
        let req1 = serde_json::to_string(&IpcRequest::new("ping")).unwrap();
        let req2 = serde_json::to_string(&IpcRequest::new("status")).unwrap();
        let combined = format!("{}\n{}\n", req1, req2);
        client.write_all(combined.as_bytes()).await.unwrap();
        client.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        let requests = server.poll_commands().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].1.command, "ping");
        assert_eq!(requests[1].1.command, "status");
    }

    #[tokio::test]
    async fn server_ignores_malformed_json() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Send malformed JSON followed by valid request
        client
            .write_all(b"not valid json\n{\"command\":\"ping\"}\n")
            .await
            .unwrap();
        client.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        let requests = server.poll_commands().unwrap();
        // Should only get the valid request
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1.command, "ping");
    }

    #[tokio::test]
    async fn connection_id_is_unique() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        // Connect two clients
        let _client1 = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        let _client2 = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        assert_eq!(server.connection_count(), 2);

        // Check that connection IDs are different
        let ids: Vec<_> = server.connections.keys().collect();
        assert_ne!(ids[0], ids[1]);
    }

    #[tokio::test]
    async fn close_connection_removes_client() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let _client = UnixStream::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        let conn_id = *server.connections.keys().next().unwrap();
        assert!(server.has_connection(conn_id));

        server.close_connection(conn_id);
        assert!(!server.has_connection(conn_id));
        assert_eq!(server.connection_count(), 0);
    }
}
