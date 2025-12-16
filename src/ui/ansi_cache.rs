use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;

use crate::ui::utils::parse_ansi_to_spans;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnsiCacheKey {
    log_id: u64,
    compact_mode: bool,
}

impl AnsiCacheKey {
    pub fn new(log_id: u64, compact_mode: bool) -> Self {
        Self { log_id, compact_mode }
    }
}

#[derive(Debug, Clone)]
pub struct CachedSpans {
    pub spans: Vec<(String, Style)>,
}

pub struct AnsiCache {
    cache: HashMap<AnsiCacheKey, CachedSpans>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

impl AnsiCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get_or_parse(
        &mut self,
        key: AnsiCacheKey,
        text: &str,
    ) -> &CachedSpans {
        if self.cache.contains_key(&key) {
            self.hits += 1;
            return self.cache.get(&key).unwrap();
        }

        self.misses += 1;

        // Evict if cache is too large (simple strategy: clear half)
        if self.cache.len() >= self.max_size {
            let to_remove: Vec<_> = self.cache.keys().take(self.max_size / 2).cloned().collect();
            for k in to_remove {
                self.cache.remove(&k);
            }
        }

        // Parse and cache
        let spans = parse_ansi_to_spans(text);
        self.cache.insert(key.clone(), CachedSpans { spans });
        self.cache.get(&key).unwrap()
    }

    pub fn to_line_with_overrides(
        cached: &CachedSpans,
        bg_color: Option<Color>,
        fg_override: Option<Color>,
    ) -> Line<'static> {
        if bg_color.is_none() && fg_override.is_none() {
            // No overrides, use cached styles directly
            let spans: Vec<Span<'static>> = cached
                .spans
                .iter()
                .map(|(content, style)| Span::styled(content.clone(), *style))
                .collect();
            Line::from(spans)
        } else {
            // Apply overrides to each span
            let spans: Vec<Span<'static>> = cached
                .spans
                .iter()
                .map(|(content, style)| {
                    let mut new_style = *style;
                    if let Some(bg) = bg_color {
                        new_style = new_style.bg(bg);
                    }
                    if let Some(fg) = fg_override {
                        new_style = new_style.fg(fg);
                    }
                    Span::styled(content.clone(), new_style)
                })
                .collect();
            Line::from(spans)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let mut cache = AnsiCache::new(100);
        let key = AnsiCacheKey::new(1, false);

        // First access - miss
        let _ = cache.get_or_parse(key.clone(), "test line");
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 0);

        // Second access - hit
        let _ = cache.get_or_parse(key.clone(), "test line");
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_different_compact_mode_is_different_key() {
        let mut cache = AnsiCache::new(100);
        let key1 = AnsiCacheKey::new(1, false);
        let key2 = AnsiCacheKey::new(1, true);

        let _ = cache.get_or_parse(key1, "full content");
        let _ = cache.get_or_parse(key2, "condensed");

        assert_eq!(cache.misses, 2);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = AnsiCache::new(10);

        // Fill cache
        for i in 0..10 {
            let key = AnsiCacheKey::new(i, false);
            cache.get_or_parse(key, "test");
        }
        assert_eq!(cache.cache.len(), 10);

        // Add one more, should trigger eviction
        let key = AnsiCacheKey::new(100, false);
        cache.get_or_parse(key, "test");

        // Should have evicted half
        assert!(cache.cache.len() <= 6);
    }

    #[test]
    fn test_line_with_overrides() {
        let cached = CachedSpans {
            spans: vec![
                ("hello ".to_string(), Style::default()),
                ("world".to_string(), Style::default().fg(Color::Red)),
            ],
        };

        // Without overrides
        let line = AnsiCache::to_line_with_overrides(&cached, None, None);
        assert_eq!(line.spans.len(), 2);

        // With background override
        let line = AnsiCache::to_line_with_overrides(&cached, Some(Color::Blue), None);
        assert_eq!(line.spans.len(), 2);
        // Each span should have blue background
    }
}
