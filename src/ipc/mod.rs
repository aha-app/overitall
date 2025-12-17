// IPC module for AI remote control
// Allows CLI clients to communicate with running TUI instance via Unix socket

pub mod protocol;
pub mod server;

pub use protocol::{IpcRequest, IpcResponse};
pub use server::{ConnectionId, IpcServer};
