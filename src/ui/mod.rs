pub mod ansi_cache;
mod app;
mod batch;
pub mod batch_cache;
mod batch_state;
mod click_regions;
mod display_state;
mod draw;
mod filter;
mod filter_state;
mod input_state;
mod navigation_state;
mod overlays;
pub mod process_colors;
mod render_cache;
mod trace_state;
mod types;
pub mod utils;
mod widgets;

// Public API
pub use app::{App, DisplayMode};
pub use batch::detect_batches_from_logs;
#[allow(unused_imports)]
pub use batch_cache::{BatchCache, BatchCacheKey};
pub use draw::draw;
pub use filter::{apply_filters, Filter, FilterType};
pub use process_colors::ProcessColors;
