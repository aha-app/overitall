use std::io;
use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::protocol::{IpcRequest, IpcResponse};

/// IPC client for sending commands to a running TUI instance
#[derive(Debug)]
pub struct IpcClient {
    reader: BufReader<UnixStream>,
}

impl IpcClient {
    /// Connect to an IPC server at the given socket path
    pub async fn connect(socket_path: impl AsRef<Path>) -> io::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self {
            reader: BufReader::new(stream),
        })
    }

    /// Send a request to the server
    pub async fn send_request(&mut self, request: &IpcRequest) -> io::Result<()> {
        let mut json = serde_json::to_string(request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        json.push('\n');

        self.reader.get_mut().write_all(json.as_bytes()).await?;
        self.reader.get_mut().flush().await?;

        Ok(())
    }

    /// Receive a response from the server
    pub async fn recv_response(&mut self) -> io::Result<IpcResponse> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;

        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "server closed connection",
            ));
        }

        serde_json::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Send a request and wait for response (convenience method)
    pub async fn call(&mut self, request: &IpcRequest) -> io::Result<IpcResponse> {
        self.send_request(request).await?;
        self.recv_response().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::server::IpcServer;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::TempDir;

    fn temp_socket_path() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sock");
        (dir, path)
    }

    #[tokio::test]
    async fn client_connects_to_server() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let _client = IpcClient::connect(&path).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let accepted = server.accept_pending().unwrap();
        assert_eq!(accepted, 1);
    }

    #[tokio::test]
    async fn client_send_request_reaches_server() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        let request = IpcRequest::new("ping");
        client.send_request(&request).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let requests = server.poll_commands().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1.command, "ping");
    }

    #[tokio::test]
    async fn client_receives_response() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Send a request to get the connection ID
        let request = IpcRequest::new("ping");
        client.send_request(&request).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let requests = server.poll_commands().unwrap();
        let conn_id = requests[0].0;

        let response = IpcResponse::ok(json!({"pong": true}));
        server.send_response(conn_id, response).await.unwrap();

        let received = client.recv_response().await.unwrap();
        assert!(received.success);
        assert_eq!(received.result, Some(json!({"pong": true})));
    }

    #[tokio::test]
    async fn client_call_sends_and_receives() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Spawn a task to handle the server side
        let handle = tokio::spawn(async move {
            // Wait for request
            tokio::time::sleep(Duration::from_millis(10)).await;
            let requests = server.poll_commands().unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].1.command, "status");

            // Send response
            let conn_id = requests[0].0;
            let response = IpcResponse::ok(json!({"running": true}));
            server.send_response(conn_id, response).await.unwrap();

            // Keep server alive until test completes
            tokio::time::sleep(Duration::from_millis(50)).await;
            // Return server to keep socket alive
            server
        });

        // Client makes call
        let request = IpcRequest::new("status");
        let response = client.call(&request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.result, Some(json!({"running": true})));

        // Wait for server task
        let _ = handle.await;
    }

    #[tokio::test]
    async fn client_handles_connection_refused() {
        let (_dir, path) = temp_socket_path();
        // Don't start a server

        let result = IpcClient::connect(&path).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn client_handles_server_disconnect() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        // Drop the server to close connections
        drop(server);

        // Try to receive - should get EOF
        let result = client.recv_response().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn client_request_with_args() {
        let (_dir, path) = temp_socket_path();
        let mut server = IpcServer::new(&path).unwrap();

        let mut client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();

        let request = IpcRequest::with_args("search", json!({"pattern": "error", "limit": 10}));
        client.send_request(&request).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let requests = server.poll_commands().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1.command, "search");
        assert_eq!(requests[0].1.args["pattern"], "error");
        assert_eq!(requests[0].1.args["limit"], 10);
    }
}
