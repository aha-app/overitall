mod app;
mod batch;
pub mod batch_cache;
mod draw;
mod filter;
mod overlays;
mod types;
mod utils;
mod widgets;

// Public API (maintains backward compatibility)
pub use app::App;
pub use batch::detect_batches_from_logs;
pub use batch_cache::{BatchCache, BatchCacheKey};
pub use draw::draw;
pub use filter::{apply_filters, Filter, FilterType};
