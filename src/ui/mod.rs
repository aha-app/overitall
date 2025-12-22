pub mod ansi_cache;
mod app;
mod batch;
pub mod batch_cache;
mod draw;
mod filter;
mod overlays;
mod types;
pub mod utils;
mod widgets;

// Public API (maintains backward compatibility)
pub use ansi_cache::{AnsiCache, AnsiCacheKey};
pub use app::{App, DisplayMode};
pub use batch::detect_batches_from_logs;
pub use batch_cache::{BatchCache, BatchCacheKey};
pub use draw::draw;
pub use filter::{apply_filters, Filter, FilterType};
