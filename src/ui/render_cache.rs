use super::ansi_cache::AnsiCache;
use super::batch_cache::BatchCache;

/// Render caches for performance optimization
pub struct RenderCache {
    /// Cache for batch detection results
    pub batch_cache: BatchCache,
    /// Cache for ANSI parsing results
    pub ansi_cache: AnsiCache,
    /// Cached batch count from last render
    pub cached_batch_count: usize,
    /// Cached batch info for status bar display: (batch_index, total_batches, line_count_in_batch)
    pub cached_batch_info: Option<(usize, usize, usize)>,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            batch_cache: BatchCache::new(),
            ansi_cache: AnsiCache::new(2000),
            cached_batch_count: 0,
            cached_batch_info: None,
        }
    }
}
