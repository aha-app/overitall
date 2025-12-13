mod app;
mod batch;
mod draw;
mod filter;
mod overlays;
mod types;
mod utils;
mod widgets;

// Public API (maintains backward compatibility)
pub use app::App;
pub use batch::detect_batches_from_logs;
pub use draw::draw;
pub use filter::{apply_filters, Filter, FilterType};
