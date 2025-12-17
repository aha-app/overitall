// IPC module for AI remote control
// Allows CLI clients to communicate with running TUI instance via Unix socket

pub mod client;
pub mod protocol;
pub mod server;

pub use client::IpcClient;
pub use protocol::{IpcRequest, IpcResponse};
pub use server::{ConnectionId, IpcServer};
