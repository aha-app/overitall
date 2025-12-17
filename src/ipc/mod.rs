// IPC module for AI remote control
// Allows CLI clients to communicate with running TUI instance via Unix socket

pub mod action;
pub mod client;
pub mod handler;
pub mod protocol;
pub mod server;
pub mod state;

pub use action::{IpcAction, IpcHandlerResult};
pub use client::IpcClient;
pub use handler::IpcCommandHandler;
pub use protocol::{IpcRequest, IpcResponse};
pub use server::{ConnectionId, IpcServer};
pub use state::{BufferStats, FilterInfo, ProcessInfo, StateSnapshot, ViewModeInfo};
