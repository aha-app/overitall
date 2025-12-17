// IPC module for AI remote control
// Allows CLI clients to communicate with running TUI instance via Unix socket

pub mod protocol;

pub use protocol::{IpcRequest, IpcResponse};
